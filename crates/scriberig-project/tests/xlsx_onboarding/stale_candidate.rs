//! Verifies an XLSX candidate cannot replace a successor based on a stale snapshot.

use std::fs;
use std::sync::Arc;

use scriberig_domain_spreadsheet::{CellValue, Formula};
use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use crate::workbook_project;

/// Rejects a validated candidate after another candidate advances its base snapshot.
#[test]
fn xlsx_candidate_from_stale_snapshot_cannot_overwrite_successor() {
    let root = workbook_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan workbook");
    let project = scan.project.id;
    let base = service.import(project).expect("import workbook").snapshot;
    let worksheet = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find_map(|entity| {
            if entity.kind != "spreadsheet.worksheet" {
                return None;
            }
            let revision = store.resolve(base, entity.entity).ok()??;
            let revision = store.get(revision).ok()??;
            let summary: serde_json::Value = serde_json::from_slice(revision.payload()).ok()?;
            (summary["name"] == "Budget").then_some(entity.entity)
        })
        .expect("Budget worksheet");
    let edit = |expression: &str| {
        service
            .propose_xlsx_cell_edit(
                project,
                base,
                worksheet,
                3,
                2,
                &CellValue::Formula(Formula {
                    expression: expression.into(),
                    cached_value: None,
                }),
            )
            .expect("propose exact-base XLSX edit")
    };
    let winner = edit("=A3*3");
    let stale = edit("=A3*4");
    let validate = |candidate: &str| {
        service
            .validate_candidate_profile_cancellable(
                candidate,
                CandidateValidationProfile::XlsxPackage,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .expect("validate staged XLSX package")
    };
    let winner = validate(&winner.id);
    let stale = validate(&stale.id);
    assert_eq!(winner.validation, CandidateValidation::Passed);
    assert_eq!(stale.validation, CandidateValidation::Passed);

    let applied = service.apply_candidate(&winner).expect("apply winner");
    assert_eq!(applied.base_snapshot, base);
    let current_before_reject = store
        .project(project)
        .expect("project state")
        .expect("project")
        .current_snapshot;
    assert_eq!(current_before_reject, Some(applied.successor_snapshot));
    assert!(service.apply_candidate(&stale).is_err());
    assert_eq!(
        store
            .project(project)
            .expect("project state after stale rejection")
            .expect("project")
            .current_snapshot,
        current_before_reject,
        "stale candidate must not advance canonical history"
    );
    let output = fs::read(root.join("workbook.xlsx")).expect("materialized winner");
    let reopened = rxls::Spreadsheet::open(&output).expect("reopen winning workbook");
    assert!(matches!(
        reopened
            .workbook()
            .sheet_by_name("Budget")
            .and_then(|sheet| sheet.cell(2, 1)),
        Some(rxls::Cell::Formula { formula, .. }) if formula == "A3*3"
    ));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
