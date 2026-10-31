//! Verifies scanned XLSX files import as workbook and sparse cell semantics.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_domain_spreadsheet::{Cell, CellValue};
use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, ProjectService, ProjectSourceChange,
    ProjectSourceChangeKind,
};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

#[path = "xlsx_onboarding/failure.rs"]
mod failure;
#[path = "xlsx_onboarding/range_edit.rs"]
mod range_edit;
#[path = "xlsx_onboarding/rename.rs"]
mod rename;
#[path = "xlsx_onboarding/stale_candidate.rs"]
mod stale_candidate;
#[path = "xlsx_onboarding/xlsx_snapshot_membership.rs"]
mod xlsx_snapshot_membership;

/// Creates a unique project with a deterministic two-sheet workbook.
/// # Returns
/// The result of the requested operation.
fn workbook_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-xlsx-onboarding-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut workbook = rxls::Workbook::new();
    let budget = workbook.add_sheet("Budget");
    budget.write_number(2, 0, 10.0);
    budget.write_formula(2, 1, "A3*2", rxls::Cell::Number(20.0));
    workbook.add_sheet("Notes").write(0, 0, "preserve me");
    fs::write(
        root.join("workbook.xlsx"),
        workbook.to_xlsx_checked().expect("write valid fixture"),
    )
    .expect("persist workbook");
    root
}

/// Scans and imports a workbook through the persisted project manifest.
#[test]
fn project_import_creates_sparse_workbook_worksheet_and_cell_entities() {
    let root = workbook_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    assert_eq!(scan.files[0].adapter, "xlsx");
    let project = scan.project.id;
    let source = fs::read(root.join("workbook.xlsx")).expect("exact source bytes");
    let manifest = store
        .project_file_descriptors(project)
        .expect("persisted scan evidence");
    assert_eq!(manifest[0].digest, *blake3::hash(&source).as_bytes());
    service.import(project).expect("import workbook");
    let entities = store
        .project_entities(project)
        .expect("persisted semantic entities");
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "spreadsheet.workbook")
            .count(),
        1
    );
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "spreadsheet.worksheet")
            .count(),
        2
    );
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "spreadsheet.cell")
            .count(),
        3
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Creates an XLSX cell candidate without changing live bytes and rejects stale bases.
#[allow(clippy::too_many_lines)]
#[test]
fn xlsx_cell_edit_candidate_is_snapshot_bound_and_does_not_write_live_file() {
    // Keep the snapshot, candidate, validation, promotion, and reopen assertions
    // together so they verify one complete transaction trace.
    let root = workbook_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let imported = service.import(project).expect("import workbook");
    let worksheet = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .filter(|entity| entity.kind == "spreadsheet.worksheet")
        .find_map(|entity| {
            let revision_id = store
                .resolve(imported.snapshot, entity.entity)
                .expect("worksheet snapshot lookup")?;
            let revision = store.get(revision_id).expect("worksheet revision lookup")?;
            let summary: serde_json::Value =
                serde_json::from_slice(revision.payload()).expect("worksheet summary");
            (summary.get("name").and_then(serde_json::Value::as_str) == Some("Budget"))
                .then_some(entity.entity)
        })
        .expect("Budget worksheet");
    let first_page = service
        .spreadsheet_cell_page(project, imported.snapshot, worksheet, None, 0, 1)
        .expect("first snapshot-bound worksheet page");
    assert_eq!(
        first_page.artifact_capability,
        Some(scriberig_project::ArtifactCapability::EditableWithOpaquePreservation)
    );
    assert_eq!(first_page.cells.len(), 1);
    assert_eq!(first_page.cells[0].row, 3);
    assert_eq!(first_page.cells[0].column, 1);
    assert_eq!(first_page.total, 2);
    assert!(first_page.has_more);
    let second_page = service
        .spreadsheet_cell_page(project, imported.snapshot, worksheet, None, 1, 1)
        .expect("second worksheet page");
    assert_eq!(second_page.cells[0].column, 2);
    assert!(!second_page.has_more);
    let cell_entities = store
        .project_entities(project)
        .expect("cell ownership")
        .into_iter()
        .filter(|entity| entity.kind == "spreadsheet.cell")
        .collect::<Vec<_>>();
    let target_cell = cell_entities
        .iter()
        .find_map(|entity| {
            let revision_id = store
                .resolve(imported.snapshot, entity.entity)
                .expect("cell snapshot lookup")?;
            let revision = store.get(revision_id).expect("cell revision lookup")?;
            let cell: serde_json::Value = serde_json::from_slice(revision.payload()).ok()?;
            (cell.get("row").and_then(serde_json::Value::as_u64) == Some(3)
                && cell.get("column").and_then(serde_json::Value::as_u64) == Some(2))
            .then_some(entity.entity)
        })
        .expect("B3 semantic cell");
    let path = root.join("workbook.xlsx");
    let original = fs::read(&path).expect("read original XLSX");
    let candidate = service
        .propose_xlsx_cell_edit(
            project,
            imported.snapshot,
            worksheet,
            3,
            2,
            &CellValue::Formula(scriberig_domain_spreadsheet::Formula {
                expression: "=A3*3".into(),
                cached_value: None,
            }),
        )
        .expect("propose exact-base cell edit");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("spreadsheet.set_cell")
    );
    let candidate = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::XlsxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate staged XLSX candidate");
    assert_eq!(candidate.validation, CandidateValidation::Passed);
    assert_eq!(candidate.base_snapshot, imported.snapshot);
    assert_eq!(candidate.old, original);
    assert_ne!(candidate.new, original);
    assert_eq!(candidate.effects.len(), 1);
    assert_eq!(
        fs::read(&path).expect("live XLSX remains unchanged"),
        original
    );
    assert_eq!(
        store
            .project(project)
            .expect("project record")
            .expect("project")
            .current_snapshot,
        Some(imported.snapshot)
    );
    let reopened = rxls::Spreadsheet::open(&candidate.new).expect("reopen candidate package");
    assert!(matches!(
        reopened
            .workbook()
            .sheet_by_name("Budget")
            .and_then(|sheet| sheet.cell(2, 1)),
        Some(rxls::Cell::Formula { formula, .. }) if formula == "A3*3"
    ));
    let corrupt = service
        .propose_project_change(
            project,
            imported.snapshot,
            vec![ProjectSourceChange {
                path: "workbook.xlsx".into(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"invalid staged package".to_vec()),
            }],
        )
        .expect("persist corrupt candidate only");
    let rejected = service
        .validate_candidate_profile_cancellable(
            &corrupt.id,
            CandidateValidationProfile::XlsxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validation returns structured rejection");
    assert!(matches!(
        rejected.validation,
        CandidateValidation::Failed { .. }
    ));
    assert!(service.apply_candidate(&rejected).is_err());
    assert_eq!(
        fs::read(&path).expect("live file unchanged after rejection"),
        original
    );
    assert!(
        service
            .propose_xlsx_cell_edit(
                project,
                scriberig_core::SnapshotId::new([0; 32]),
                worksheet,
                3,
                2,
                &CellValue::Text("stale".into()),
            )
            .is_err()
    );
    let applied = service
        .apply_candidate(&candidate)
        .expect("apply validated XLSX candidate");
    assert_eq!(applied.base_snapshot, imported.snapshot);
    assert_ne!(applied.successor_snapshot, imported.snapshot);
    assert_eq!(
        store
            .change_provenance(applied.successor_snapshot)
            .expect("read spreadsheet snapshot provenance")
            .expect("applied spreadsheet change")
            .semantic_operation
            .as_deref(),
        Some("spreadsheet.set_cell")
    );
    let changes = store
        .diff_snapshot_revisions(imported.snapshot, applied.successor_snapshot)
        .expect("semantic snapshot diff");
    let changed_entities = changes
        .iter()
        .map(|change| match change {
            scriberig_store::SnapshotDiff::Added { entity, .. }
            | scriberig_store::SnapshotDiff::Changed { entity, .. }
            | scriberig_store::SnapshotDiff::Removed { entity, .. } => *entity,
        })
        .collect::<Vec<_>>();
    assert!(changed_entities.contains(&target_cell));
    assert!(
        cell_entities
            .iter()
            .filter(|entity| entity.entity != target_cell)
            .all(|entity| !changed_entities.contains(&entity.entity))
    );
    let target_revision = store
        .resolve(applied.successor_snapshot, target_cell)
        .expect("target in successor")
        .expect("target revision");
    let target_semantics: Cell = serde_json::from_slice(
        store
            .get(target_revision)
            .expect("target revision lookup")
            .expect("target revision")
            .payload(),
    )
    .expect("target cell semantics");
    assert!(matches!(
        target_semantics.value,
        CellValue::Formula(formula) if formula.expression == "=A3*3"
    ));
    let target_range = scriberig_domain_spreadsheet::CellRange {
        first_row: 3,
        first_column: 2,
        last_row: 3,
        last_column: 2,
    };
    let historical_page = service
        .spreadsheet_cell_page(
            project,
            imported.snapshot,
            worksheet,
            Some(&target_range),
            0,
            1,
        )
        .expect("read historical cell revision");
    assert!(matches!(
        &historical_page.cells[0].value,
        CellValue::Formula(formula) if formula.expression == "=A3*2"
    ));
    let current_page = service
        .spreadsheet_cell_page(
            project,
            applied.successor_snapshot,
            worksheet,
            Some(&target_range),
            0,
            1,
        )
        .expect("read changed cell revision");
    assert!(matches!(
        &current_page.cells[0].value,
        CellValue::Formula(formula) if formula.expression == "=A3*3"
    ));
    let materialized = fs::read(&path).expect("read applied workbook");
    assert_ne!(materialized, original);
    let descriptor = store
        .project_file_descriptors(project)
        .expect("applied manifest")
        .into_iter()
        .find(|entry| entry.relative_path == "workbook.xlsx")
        .expect("applied XLSX evidence");
    assert_eq!(descriptor.digest, *blake3::hash(&materialized).as_bytes());
    let reopened = rxls::Spreadsheet::open(&materialized).expect("reopen applied workbook");
    assert!(matches!(
        reopened
            .workbook()
            .sheet_by_name("Budget")
            .and_then(|sheet| sheet.cell(2, 1)),
        Some(rxls::Cell::Formula { formula, .. }) if formula == "A3*3"
    ));
    assert!(matches!(
        reopened
            .workbook()
            .sheet_by_name("Notes")
            .and_then(|sheet| sheet.cell(0, 0)),
        Some(rxls::Cell::Text(value)) if value == "preserve me"
    ));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
