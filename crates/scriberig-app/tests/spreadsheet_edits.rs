//! Verifies authorized spreadsheet edits create candidates without live writes.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_agent_core::Tool as _;
use scriberig_app::agent_tools::{SPREADSHEET_CELL_EDIT_TOOL, SpreadsheetCellEditTool};
use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, Capability, CapabilityAction,
    CapabilityConstraints, CapabilityContext, CapabilityScope, OperationKind,
};
use scriberig_domain_spreadsheet::{CellValue, Formula};
use scriberig_project::{CandidateValidation, ProjectService};
use scriberig_store_sqlite::SqliteStore;

/// Creates a scoped Propose context for the imported project snapshot.
///
/// # Parameters
/// - `project`: Project identity to authorize.
/// - `snapshot`: Exact canonical base snapshot.
///
/// # Returns
/// A context restricted to one project's domain operations.
fn propose_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints {
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "This application integration test verifies exact-snapshot authorization, typed candidate construction, and unchanged source bytes end to end."
)]
/// Verifies exact-snapshot authorization and candidate-only XLSX edits.
#[test]
fn spreadsheet_edit_requires_propose_grant_and_keeps_live_bytes_unchanged() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-xlsx-edit-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut workbook = rxls::Workbook::new();
    workbook.add_sheet("Budget").write_number(0, 0, 10.0);
    let bytes = workbook.to_xlsx_checked().expect("serialize workbook");
    let path = root.join("budget.xlsx");
    fs::write(&path, &bytes).expect("write workbook");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan workbook");
    let imported = projects.import(scan.project.id).expect("import workbook");
    let worksheet = store
        .project_entities(scan.project.id)
        .expect("semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "spreadsheet.worksheet")
        .expect("worksheet")
        .entity;
    let service = ApplicationService::new(()).with_project_service(projects);
    let exact = propose_context(scan.project.id, imported.snapshot);
    let denied = service.propose_spreadsheet_cell_edit(
        &ApplicationContext::default().with_snapshot(imported.snapshot),
        scan.project.id,
        worksheet,
        1,
        1,
        &CellValue::Formula(Formula {
            expression: "=A1*3".into(),
            cached_value: None,
        }),
    );
    assert!(matches!(
        denied,
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let candidate = service
        .propose_spreadsheet_cell_edit(
            &exact,
            scan.project.id,
            worksheet,
            1,
            2,
            &CellValue::Formula(Formula {
                expression: "=A1*3".into(),
                cached_value: None,
            }),
        )
        .expect("authorized candidate");
    assert_eq!(candidate.project, scan.project.id);
    assert_eq!(candidate.base_snapshot, imported.snapshot);
    assert_eq!(candidate.path, "budget.xlsx");
    assert_eq!(candidate.validation, CandidateValidation::Pending);
    assert_eq!(fs::read(&path).expect("read live source"), bytes);

    let tool = SpreadsheetCellEditTool::new(service.clone(), scan.project.id);
    let cell = store
        .project_entities(scan.project.id)
        .expect("semantic cell ownership")
        .into_iter()
        .find(|entity| entity.kind == "spreadsheet.cell")
        .expect("existing semantic cell")
        .entity;
    let exact_cell_context = ApplicationContext {
        snapshot: Some(imported.snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Entity(cell),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints {
                snapshot: Some(imported.snapshot),
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    };
    service
        .propose_spreadsheet_semantic_cell_edit(
            &exact_cell_context,
            scan.project.id,
            cell,
            &CellValue::Text("exact target".to_owned()),
        )
        .expect("exact cell grant authorizes the selected target");
    assert!(matches!(
        service.propose_spreadsheet_semantic_cell_edit(
            &exact_cell_context,
            scan.project.id,
            scriberig_core::EntityId::new(),
            &CellValue::Text("other target".to_owned()),
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let tool_result = tool
        .invoke_named(
            &exact,
            SPREADSHEET_CELL_EDIT_TOOL,
            &serde_json::json!({
                "cell": cell.to_string(),
                "value": {
                    "kind": "text",
                    "value": "agent edit"
                }
            }),
        )
        .expect("authorized agent candidate");
    assert_eq!(tool_result.entity, cell);
    assert_eq!(tool_result.snapshot, Some(imported.snapshot));
    assert_eq!(
        tool_result.object_type.as_deref(),
        Some("spreadsheet.candidate")
    );
    assert_eq!(tool_result.handles.len(), 1);
    assert_eq!(fs::read(&path).expect("agent keeps live source"), bytes);

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
