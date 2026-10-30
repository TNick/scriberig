//! Verifies the integrated provider tool creates exact-base spreadsheet candidates.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_agent_core::Tool as _;
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, Capability,
    CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope, OperationKind,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;

/// Verifies the cataloged provider tool returns a candidate without writing the live workbook.
#[test]
fn spreadsheet_agent_tool_uses_propose_grant_and_preserves_live_bytes() {
    let root = std::env::temp_dir().join(format!("scriberig-agent-xlsx-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("project root");
    let mut workbook = rxls::Workbook::new();
    workbook.add_sheet("Budget").write_number(0, 0, 10.0);
    let bytes = workbook.to_xlsx_checked().expect("serialize workbook");
    let path = root.join("budget.xlsx");
    fs::write(&path, &bytes).expect("write workbook");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan workbook");
    let imported = projects.import(scan.project.id).expect("import workbook");
    let worksheet = store
        .project_entities(scan.project.id)
        .expect("project semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "spreadsheet.worksheet")
        .expect("worksheet entity")
        .entity;
    let cell = store
        .project_entities(scan.project.id)
        .expect("project semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "spreadsheet.cell")
        .expect("cell entity")
        .entity;
    let context = agent_context(scan.project.id, imported.snapshot);
    let service = ApplicationService::new(ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    ))
    .with_project_service(projects.clone());
    let tool = LocalAgentTool::new(&service);
    let result = tool
        .invoke_named(
            &context,
            "spreadsheet.set_cell_value",
            &serde_json::json!({
                "project": scan.project.id.to_string(),
                "cell": cell.to_string(),
                "value": {"kind": "number", "value": "12.5"}
            }),
        )
        .expect("semantic candidate from provider tool");

    assert_eq!(result.entity, cell);
    assert_eq!(result.snapshot, Some(imported.snapshot));
    assert_eq!(result.object_type.as_deref(), Some("spreadsheet.candidate"));
    assert_eq!(result.handles.len(), 1);
    let rename = tool
        .invoke_named(
            &context,
            "spreadsheet.rename_sheet",
            &serde_json::json!({
                "project": scan.project.id.to_string(),
                "worksheet": worksheet.to_string(),
                "new_name": "Plan"
            }),
        )
        .expect("semantic rename candidate from provider tool");
    assert_eq!(rename.entity, worksheet);
    assert_eq!(rename.snapshot, Some(imported.snapshot));
    assert_eq!(rename.handles.len(), 1);
    let durable_rename = projects
        .candidate(&rename.handles[0])
        .expect("load rename candidate")
        .expect("durable rename candidate");
    assert_eq!(
        durable_rename.semantic_operation.as_deref(),
        Some("spreadsheet.rename_sheet")
    );
    let tui_service = scriberig_tui::service::ApplicationOperationAdapter::new(service.clone())
        .with_project_workflow(projects.clone());
    let tui_candidate = scriberig_tui::service::TuiService::propose_spreadsheet_edit(
        &tui_service,
        &context,
        &scriberig_app::SpreadsheetCellEditQuery {
            project: scan.project.id,
            snapshot: imported.snapshot,
            worksheet,
            row: 1,
            column: 1,
            value: scriberig_domain_spreadsheet::CellValue::Number("13.5".to_owned()),
        },
    )
    .expect("TUI adapter creates the shared exact-base candidate");
    assert_eq!(tui_candidate.project, scan.project.id);
    assert_eq!(tui_candidate.base_snapshot, imported.snapshot);
    assert_eq!(tui_candidate.path, "budget.xlsx");
    assert_eq!(
        fs::read(&path).expect("live workbook remains readable"),
        bytes
    );

    drop(tool);
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project root");
}

/// Builds the exact-snapshot catalog and project Propose grants for the fixture.
///
/// # Parameters
/// - `project`: Project receiving the candidate.
/// - `snapshot`: Exact imported snapshot.
///
/// # Returns
/// Trusted application context for the local semantic agent.
fn agent_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([
            Capability {
                scope: CapabilityScope::Project,
                actions: BTreeSet::from([
                    CapabilityAction::Custom("spreadsheet.set_cell_value".to_owned()),
                    CapabilityAction::Custom("spreadsheet.rename_sheet".to_owned()),
                ]),
                constraints: CapabilityConstraints::unrestricted(),
            },
            Capability {
                scope: CapabilityScope::Custom(format!("project:{project}")),
                actions: BTreeSet::from([CapabilityAction::Propose]),
                constraints: CapabilityConstraints {
                    operation: Some(OperationKind::DomainOperation),
                    ..CapabilityConstraints::unrestricted()
                },
            },
        ]),
        ..ApplicationContext::default()
    }
}
