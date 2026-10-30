//! Verifies worksheet renames use exact-base candidates and shared promotion.

use std::fs;
use std::sync::Arc;

use scriberig_project::{CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use crate::workbook_project;

/// Renames a worksheet through validation and apply while preserving identity.
#[test]
fn xlsx_sheet_rename_updates_snapshot_and_retains_operation_provenance() {
    let root = workbook_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan workbook");
    let project = scan.project.id;
    let base = service.import(project).expect("import workbook").snapshot;
    let worksheet = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| {
            if entity.kind != "spreadsheet.worksheet" {
                return false;
            }
            let revision = store
                .resolve(base, entity.entity)
                .expect("resolve worksheet");
            let Some(revision) = revision else {
                return false;
            };
            let value = store.get(revision).expect("read worksheet revision");
            value.is_some_and(|revision| {
                serde_json::from_slice::<serde_json::Value>(revision.payload())
                    .is_ok_and(|value| value["name"] == "Budget")
            })
        })
        .expect("Budget worksheet");
    let path = root.join("workbook.xlsx");
    let original = fs::read(&path).expect("read exact source bytes");
    let candidate = service
        .propose_xlsx_sheet_rename(project, base, worksheet.entity, "Plan")
        .expect("propose worksheet rename");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("spreadsheet.rename_sheet")
    );
    assert_eq!(candidate.old, original);
    assert_ne!(candidate.new, original);
    assert_eq!(fs::read(&path).expect("live file before apply"), original);
    assert_eq!(
        store
            .project(project)
            .expect("project record")
            .expect("project")
            .current_snapshot,
        Some(base)
    );

    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::XlsxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate renamed package");
    assert_eq!(
        validated.validation,
        scriberig_project::CandidateValidation::Passed
    );
    let applied = service
        .apply_candidate(&validated)
        .expect("apply renamed workbook");
    assert_eq!(
        store
            .change_provenance(applied.successor_snapshot)
            .expect("snapshot change provenance")
            .expect("rename provenance")
            .semantic_operation
            .as_deref(),
        Some("spreadsheet.rename_sheet")
    );
    let renamed = store
        .resolve(applied.successor_snapshot, worksheet.entity)
        .expect("resolve stable worksheet identity")
        .expect("renamed worksheet revision");
    let revision = store
        .get(renamed)
        .expect("load renamed worksheet")
        .expect("renamed worksheet payload");
    let summary: serde_json::Value =
        serde_json::from_slice(revision.payload()).expect("decode renamed worksheet");
    assert_eq!(summary["name"], "Plan");
    let changes = store
        .diff_snapshot_revisions(base, applied.successor_snapshot)
        .expect("semantic rename diff");
    assert!(changes.iter().any(|change| matches!(
        change,
        scriberig_store::SnapshotDiff::Changed { entity, .. } if *entity == worksheet.entity
    )));
    assert_renamed_package_content(&path);

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Verifies the materialized workbook contains only the renamed sheet identity.
///
/// # Parameters
/// - `path`: Materialized workbook path.
fn assert_renamed_package_content(path: &std::path::Path) {
    let reopened = rxls::Spreadsheet::open(&fs::read(path).expect("materialized XLSX"))
        .expect("reopen materialized workbook");
    assert!(reopened.workbook().sheet_by_name("Plan").is_some());
    assert!(reopened.workbook().sheet_by_name("Budget").is_none());
    assert_eq!(
        reopened
            .workbook()
            .sheet_by_name("Plan")
            .and_then(|sheet| sheet.cell(2, 1)),
        Some(&rxls::Cell::Formula {
            formula: "A3*2".to_owned(),
            cached: Box::new(rxls::Cell::Number(20.0)),
        })
    );
}
