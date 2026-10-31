//! Verifies worksheet membership indexes remain bound to their own snapshots.

use std::fs;
use std::sync::Arc;

use scriberig_domain_spreadsheet::CellValue;
use scriberig_project::{CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use super::workbook_project;

/// Verifies adding a cell updates only successor snapshot membership.
#[test]
fn added_cell_appears_only_in_successor_snapshot_membership() {
    let root = workbook_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan workbook project");
    let project = scan.project.id;
    let imported = service.import(project).expect("import workbook");
    let worksheet = store
        .project_entities(project)
        .expect("project entities")
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
                .then_some(entity)
        })
        .expect("worksheet")
        .entity;
    let before = service
        .spreadsheet_cell_page(project, imported.snapshot, worksheet, None, 0, 10)
        .expect("base snapshot cells");
    assert_eq!(before.total, 2);

    let candidate = service
        .propose_xlsx_cell_edit(
            project,
            imported.snapshot,
            worksheet,
            20,
            7,
            &CellValue::Text("new member".into()),
        )
        .expect("propose new cell");
    let candidate = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::XlsxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate new cell candidate");
    let applied = service
        .apply_candidate(&candidate)
        .expect("apply new cell candidate");

    let historical = service
        .spreadsheet_cell_page(project, imported.snapshot, worksheet, None, 0, 10)
        .expect("historical cell membership");
    let successor = service
        .spreadsheet_cell_page(project, applied.successor_snapshot, worksheet, None, 0, 10)
        .expect("successor cell membership");
    assert_eq!(historical.total, 2);
    assert_eq!(historical.cells.len(), 2);
    assert_eq!(successor.total, 3);
    assert_eq!(successor.cells.len(), 3);
    assert!(
        successor
            .cells
            .iter()
            .any(|cell| cell.row == 20 && cell.column == 7)
    );

    drop(service);
    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove workbook project");
}

/// Verifies persisted validation cannot authorize different candidate output bytes.
#[test]
fn changed_durable_output_fails_exact_validation_digest_check() {
    let root = workbook_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan workbook project");
    let project = scan.project.id;
    let imported = service.import(project).expect("import workbook");
    let worksheet = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "spreadsheet.worksheet")
        .expect("worksheet")
        .entity;
    let candidate = service
        .propose_xlsx_cell_edit(
            project,
            imported.snapshot,
            worksheet,
            20,
            7,
            &CellValue::Text("validated output".into()),
        )
        .expect("propose cell edit");
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::XlsxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate candidate");
    assert!(validated.validation_digest.is_some());

    let mut durable = store
        .project_candidate(&candidate.id)
        .expect("load durable candidate")
        .expect("candidate exists");
    durable.new.push(0);
    durable.effects[0]
        .new
        .as_mut()
        .expect("updated effect output")
        .push(0);
    store
        .put_project_candidate(&durable)
        .expect("persist simulated output corruption");
    let corrupted = service
        .candidate(&candidate.id)
        .expect("reload durable candidate")
        .expect("candidate exists");
    assert_eq!(corrupted.validation_digest, validated.validation_digest);
    let error = service
        .apply_candidate(&corrupted)
        .expect_err("changed output must not reuse old validation");
    assert_eq!(error.code, "validation_digest_mismatch");
    assert_eq!(
        fs::read(root.join("workbook.xlsx")).expect("live workbook unchanged"),
        candidate.old
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove workbook project");
}
