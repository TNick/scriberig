//! Verifies bounded multi-cell XLSX edits remain one durable semantic candidate.

use std::fs;
use std::sync::Arc;

use scriberig_domain_spreadsheet::{Cell, CellId, CellValue, Formula};
use scriberig_domain_xlsx::XlsxCellEdit;
use scriberig_project::{CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

#[allow(
    clippy::too_many_lines,
    reason = "This integration test verifies the complete multi-cell candidate lifecycle with persisted diffs."
)]
/// Applies two exact-base cell edits and checks one semantic candidate lifecycle.
#[test]
fn xlsx_range_edit_persists_each_typed_diff_and_applies_as_one_candidate() {
    let root = super::workbook_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let original_snapshot = service.import(project).expect("import workbook").snapshot;
    let original_bytes = fs::read(root.join("workbook.xlsx")).expect("source bytes");
    let worksheet = store
        .project_entities(project)
        .expect("worksheet entities")
        .into_iter()
        .filter(|entity| entity.kind == "spreadsheet.worksheet")
        .find_map(|entity| {
            let revision = store
                .resolve(original_snapshot, entity.entity)
                .expect("worksheet revision lookup")?;
            let record = store.get(revision).expect("worksheet revision")?;
            let value: serde_json::Value =
                serde_json::from_slice(record.payload()).expect("worksheet metadata");
            (value.get("name").and_then(serde_json::Value::as_str) == Some("Budget"))
                .then_some(entity.entity)
        })
        .expect("Budget worksheet");
    let edits = [
        XlsxCellEdit {
            row: 3,
            column: 1,
            value: CellValue::Number("11".to_owned()),
        },
        XlsxCellEdit {
            row: 3,
            column: 2,
            value: CellValue::Formula(Formula {
                expression: "=A3*3".to_owned(),
                cached_value: None,
            }),
        },
    ];

    let candidate = service
        .propose_xlsx_range_edit(project, original_snapshot, worksheet, &edits)
        .expect("propose range edit");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("spreadsheet.set_range")
    );
    assert_eq!(candidate.semantic_changes.len(), 2);
    let workbook = scriberig_domain_xlsx::semantic_workbook_id(
        store
            .project_file_descriptors(project)
            .expect("source descriptors")[0]
            .entity,
    );
    assert_eq!(
        candidate.semantic_changes[0].target,
        CellId::new(workbook, worksheet, 3, 1).entity()
    );
    assert_eq!(
        candidate.semantic_changes[1].target,
        CellId::new(workbook, worksheet, 3, 2).entity()
    );
    assert_eq!(candidate.semantic_changes[0].property, "cell.value");
    assert_eq!(
        candidate.semantic_changes[0].before,
        serde_json::json!({"Number": "10"})
    );
    assert_eq!(
        candidate.semantic_changes[0].after,
        serde_json::json!({"Number": "11"})
    );
    assert_eq!(
        candidate.semantic_changes[1].before,
        serde_json::json!({
            "Formula": {"expression": "=A3*2", "cached_value": {"Number": "20"}}
        })
    );
    assert_eq!(
        candidate.semantic_changes[1].after,
        serde_json::json!({
            "Formula": {"expression": "=A3*3", "cached_value": null}
        })
    );
    assert!(
        service
            .propose_xlsx_range_edit(
                project,
                original_snapshot,
                worksheet,
                &[edits[0].clone(), edits[0].clone()],
            )
            .is_err()
    );
    assert_eq!(
        service
            .candidate(&candidate.id)
            .expect("reload candidate")
            .expect("durable candidate")
            .semantic_changes,
        candidate.semantic_changes
    );
    assert_eq!(
        fs::read(root.join("workbook.xlsx")).expect("live bytes"),
        original_bytes
    );
    assert_eq!(
        service.workspace(project).expect("workspace").snapshot,
        original_snapshot
    );
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::XlsxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate range edit");
    let applied = service
        .apply_candidate(&validated)
        .expect("apply range edit");
    assert_eq!(
        store
            .change_provenance(applied.successor_snapshot)
            .expect("range provenance")
            .expect("applied range")
            .semantic_operation
            .as_deref(),
        Some("spreadsheet.set_range")
    );
    let cell_entities = store
        .project_entities(project)
        .expect("cell entities")
        .into_iter()
        .filter(|entity| entity.kind == "spreadsheet.cell")
        .map(|entity| entity.entity)
        .collect::<Vec<_>>();
    let changed = store
        .diff_snapshot_revisions(original_snapshot, applied.successor_snapshot)
        .expect("cell revision changes")
        .into_iter()
        .map(|change| match change {
            scriberig_store::SnapshotDiff::Added { entity, .. }
            | scriberig_store::SnapshotDiff::Changed { entity, .. }
            | scriberig_store::SnapshotDiff::Removed { entity, .. } => entity,
        })
        .filter(|entity| cell_entities.contains(entity))
        .collect::<Vec<_>>();
    assert_eq!(changed.len(), 2);
    assert!(
        candidate
            .semantic_changes
            .iter()
            .all(|change| changed.contains(&change.target))
    );
    let updated_cells = store.project_entities(project).expect("updated cells");
    for target in candidate
        .semantic_changes
        .iter()
        .map(|change| change.target)
    {
        let revision = store
            .resolve(applied.successor_snapshot, target)
            .expect("target cell")
            .expect("cell revision");
        let cell: Cell = serde_json::from_slice(
            store
                .get(revision)
                .expect("cell lookup")
                .expect("cell")
                .payload(),
        )
        .expect("decode cell");
        assert!(
            updated_cells
                .iter()
                .any(|entity| entity.entity == cell.id.entity())
        );
    }

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
