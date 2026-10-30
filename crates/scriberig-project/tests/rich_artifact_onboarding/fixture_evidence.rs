//! Verifies rich-project fixture generation and exact XLSX semantic anchors.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use scriberig_core::{EntityId, SnapshotId};
use scriberig_domain_spreadsheet::{Cell, CellRange, CellValue};

#[path = "fixture_evidence/mixed_assertions.rs"]
mod mixed_assertions;
pub(super) use mixed_assertions::{
    assert_candidate_target, assert_document_paragraph_change, assert_docx_opaque_anchor,
    assert_live_file, paragraph_by_text, worksheet_by_name,
};
use scriberig_domain_xlsx::XlsxImport;
use scriberig_project::{ProjectService, ScanFile};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileDescriptor, SqliteStore};

/// Regenerates the stable rich-project fixture from the checked-in source builders.
#[test]
#[ignore = "run explicitly when updating the rich-project binary fixtures"]
fn generate_checked_in_rich_project_fixture() {
    let destination = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project");
    super::support::generate_mixed_fixture(&destination);
}

/// Verifies the synthetic fixture generator produces identical file bytes twice.
#[test]
fn rich_project_fixture_generation_is_byte_deterministic() {
    let first = std::env::temp_dir().join(format!(
        "scriberig-rich-fixture-first-{}",
        scriberig_core::EntityId::new()
    ));
    let second = std::env::temp_dir().join(format!(
        "scriberig-rich-fixture-second-{}",
        scriberig_core::EntityId::new()
    ));
    super::support::generate_mixed_fixture(&first);
    super::support::generate_mixed_fixture(&second);
    let files = |directory: &Path| -> BTreeMap<String, Vec<u8>> {
        fs::read_dir(directory)
            .expect("read generated fixture")
            .map(|entry| {
                let entry = entry.expect("read generated fixture entry");
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path()).expect("read generated fixture bytes"),
                )
            })
            .collect()
    };
    assert_eq!(files(&first), files(&second));
    fs::remove_dir_all(first).expect("remove first generated fixture");
    fs::remove_dir_all(second).expect("remove second generated fixture");
}

/// Asserts adapter selection for all inputs in the mixed rich-project fixture.
///
/// # Parameters
/// - `files`: Persisted scan descriptors to inspect.
///
/// # Panics
/// Panics when a known fixture extension is classified by another adapter.
pub(super) fn assert_scan_adapters(files: &[ScanFile]) {
    assert_eq!(files.len(), 12);
    for (path, adapter) in [
        ("source.rs", "rust"),
        ("budget.xlsx", "xlsx"),
        ("report.docx", "docx"),
        ("drawing.dxf", "dxf"),
        ("project.qgs", "qgis"),
        ("project.qgz", "qgis"),
        ("unresolved.qgs", "qgis"),
        ("parcels.gpkg", "geopackage"),
        ("roads.shp", "shapefile"),
        ("roads.dbf", "binary"),
        ("roads.shx", "binary"),
        ("roads.prj", "binary"),
    ] {
        assert!(
            files
                .iter()
                .any(|file| file.relative_path == path && file.adapter == adapter),
            "{path} should use {adapter} adapter"
        );
    }
}

/// Asserts the mixed project imported its Rust source as a named function.
///
/// - `entities`: Semantic ownership records from the imported project.
///
/// # Panics
/// Panics when `source.rs` does not produce the expected function symbol.
pub(super) fn assert_rust_source_imported(entities: &[ProjectEntityRecord]) {
    assert!(entities.iter().any(|entity| {
        entity.kind == "rust.function"
            && entity.name.as_deref() == Some("parcel_area")
            && entity.relative_path == "source.rs"
    }));
}

/// Asserts exact source hashes against every persisted fixture descriptor.
///
/// # Parameters
/// - `root`: Fresh copied fixture directory.
/// - `files`: Durable source descriptors recorded by project scan.
///
/// # Panics
/// Panics when any persisted digest differs from exact source bytes.
pub(super) fn assert_source_digests(root: &Path, files: &[ProjectFileDescriptor]) {
    for descriptor in files {
        let bytes = fs::read(root.join(&descriptor.relative_path)).expect("source bytes");
        assert_eq!(descriptor.digest, *blake3::hash(&bytes).as_bytes());
    }
}

/// Opens the checked-in XLSX fixture and asserts all documented source anchors.
///
/// # Parameters
/// - `bytes`: Exact checked-in XLSX package bytes.
///
/// # Panics
/// Panics when the selected parser rejects the package or an anchor is absent.
pub(super) fn assert_original_xlsx(bytes: &[u8]) {
    let imported = scriberig_domain_xlsx::import_xlsx(bytes, scriberig_core::EntityId::new())
        .expect("open checked-in XLSX fixture before editing");
    assert!(imported.editable);
    let budget = imported
        .worksheets
        .iter()
        .find(|worksheet| worksheet.name == "Budget")
        .expect("fixture Budget worksheet");
    assert!(budget.cells.iter().any(|cell| {
        cell.row == 3 && cell.column == 1 && cell.value == CellValue::Number("10".into())
    }));
    assert!(budget.cells.iter().any(|cell| {
        cell.row == 3
            && cell.column == 2
            && matches!(&cell.value, CellValue::Formula(formula) if formula.expression == "=A3*2")
    }));
    assert!(budget.cells.iter().any(|cell| {
        cell.row == 3 && cell.column == 3 && cell.value == CellValue::Text("existing".into())
    }));
    assert!(
        imported
            .workbook
            .defined_names
            .iter()
            .any(|name| name.name == "BudgetStart")
    );
    let notes = imported
        .worksheets
        .iter()
        .find(|worksheet| worksheet.name == "Notes")
        .expect("fixture Notes worksheet");
    assert!(notes.cells.iter().any(|cell| {
        cell.row == 1 && cell.column == 1 && cell.value == CellValue::Text("preserve me".into())
    }));
}

/// Asserts untargeted XLSX cell, worksheet, and defined-name preservation.
///
/// # Parameters
/// - `imported`: Reopened package after applying the candidate.
///
/// # Panics
/// Panics when an expected original semantic anchor is missing or changed.
pub(super) fn assert_materialized_xlsx(imported: &XlsxImport) {
    let budget = imported
        .worksheets
        .iter()
        .find(|worksheet| worksheet.name == "Budget")
        .expect("Budget worksheet after edit");
    assert!(budget.cells.iter().any(|cell| {
        cell.row == 3 && cell.column == 1 && cell.value == CellValue::Number("10".into())
    }));
    assert!(budget.cells.iter().any(|cell| {
        cell.row == 3
            && cell.column == 2
            && matches!(&cell.value, CellValue::Formula(formula) if formula.expression == "=A3*3")
    }));
    assert!(budget.cells.iter().any(|cell| {
        cell.row == 3 && cell.column == 3 && cell.value == CellValue::Text("existing".into())
    }));
    assert!(
        imported
            .workbook
            .defined_names
            .iter()
            .any(|name| name.name == "BudgetStart")
    );
    let notes = imported
        .worksheets
        .iter()
        .find(|worksheet| worksheet.name == "Notes")
        .expect("Notes worksheet remains");
    assert!(notes.cells.iter().any(|cell| {
        cell.row == 1 && cell.column == 1 && cell.value == CellValue::Text("preserve me".into())
    }));
}

/// Proves a mixed-project formula edit changes only the exact cell revision.
///
/// - `service`: Project query service used for snapshot-bound reads.
/// - `store`: Revision store resolving exact semantic cell revisions.
/// - `project`: Project containing the mixed fixture.
/// - `base`: Snapshot before the spreadsheet candidate.
/// - `successor`: Snapshot after the candidate was applied.
/// - `worksheet`: Exact Budget worksheet identity.
pub(super) fn assert_formula_candidate_change(
    service: &ProjectService,
    store: &SqliteStore,
    project: scriberig_core::ProjectId,
    base: SnapshotId,
    successor: SnapshotId,
    worksheet: EntityId,
) {
    let before = formula_b3(service, project, base, worksheet);
    let after = formula_b3(service, project, successor, worksheet);
    assert_eq!(before.id, after.id, "B3 semantic identity remains stable");
    assert!(matches!(
        before.value,
        CellValue::Formula(formula) if formula.expression == "=A3*2"
    ));
    assert!(matches!(
        after.value,
        CellValue::Formula(formula) if formula.expression == "=A3*3"
    ));
    assert_ne!(
        store
            .resolve(base, before.id.entity())
            .expect("base B3 revision lookup"),
        store
            .resolve(successor, after.id.entity())
            .expect("successor B3 revision lookup"),
        "formula update creates a new semantic cell revision"
    );
}

/// Checks the exact materialized bytes match persisted source digests.
///
/// - `root`: Fresh copied mixed-project directory.
/// - `document`: Persisted DOCX source descriptor after apply.
/// - `workbook`: Persisted XLSX source descriptor after apply.
pub(super) fn assert_materialized_digests(
    root: &Path,
    document: &ProjectFileDescriptor,
    workbook: &ProjectFileDescriptor,
) {
    for source in [document, workbook] {
        let bytes = fs::read(root.join(&source.relative_path)).expect("materialized artifact");
        assert_eq!(source.digest, *blake3::hash(&bytes).as_bytes());
    }
}

/// Reads the one exact B3 formula cell from a requested worksheet snapshot.
///
/// - `service`: Project query service.
/// - `project`: Project containing the worksheet.
/// - `snapshot`: Exact immutable snapshot to read.
/// - `worksheet`: Semantic Budget worksheet identity.
///
/// # Returns
/// The semantic B3 cell from the bounded single-cell page.
fn formula_b3(
    service: &ProjectService,
    project: scriberig_core::ProjectId,
    snapshot: SnapshotId,
    worksheet: EntityId,
) -> Cell {
    let range = CellRange {
        first_row: 3,
        first_column: 2,
        last_row: 3,
        last_column: 2,
    };
    let page = service
        .spreadsheet_cell_page(project, snapshot, worksheet, Some(&range), 0, 1)
        .expect("read exact B3 semantic cell");
    assert_eq!(page.snapshot, snapshot);
    assert_eq!(page.cells.len(), 1);
    page.cells.into_iter().next().expect("B3 cell")
}

/// Verifies resolved and unresolved QGIS references at the imported snapshot.
///
/// # Parameters
/// - `store`: Durable project semantic store.
/// - `snapshot`: Exact imported snapshot.
/// - `entities`: Project ownership records.
/// - `dataset_id`: Expected `GeoPackage` dataset identity.
/// - `layer_id`: Expected `GeoPackage` layer identity.
///
/// # Panics
/// Panics when the exact snapshot contains an unexpected reference resolution.
pub(super) fn assert_qgis_references(
    store: &SqliteStore,
    snapshot: SnapshotId,
    entities: &[ProjectEntityRecord],
    dataset_id: EntityId,
    layer_id: EntityId,
) {
    let reference = entities
        .iter()
        .filter(|entity| {
            entity.kind == "qgis.layer_reference" && entity.relative_path == "project.qgs"
        })
        .find_map(|entity| {
            let revision = store.resolve(snapshot, entity.entity).ok().flatten()?;
            let record = store.get(revision).ok().flatten()?;
            let reference: scriberig_domain_qgis::QgisLayerReference =
                serde_json::from_slice(record.payload()).ok()?;
            (reference.name == "Parcels").then_some(reference)
        })
        .expect("resolve Parcels QGIS reference");
    assert_eq!(
        reference.dataset,
        Some(dataset_id),
        "QGIS reference should resolve to the imported GeoPackage dataset"
    );
    assert_eq!(reference.layer, Some(layer_id));
    assert_eq!(reference.resolution_diagnostic, None);

    let unresolved_entity = entities
        .iter()
        .find(|entity| {
            entity.kind == "qgis.layer_reference" && entity.relative_path == "unresolved.qgs"
        })
        .expect("unresolved QGIS reference remains in project");
    let unresolved_revision = store
        .resolve(snapshot, unresolved_entity.entity)
        .expect("resolve unresolved reference")
        .expect("unresolved reference revision");
    let unresolved_record = store
        .get(unresolved_revision)
        .expect("read unresolved reference")
        .expect("unresolved reference payload");
    let unresolved: scriberig_domain_qgis::QgisLayerReference =
        serde_json::from_slice(unresolved_record.payload()).expect("decode unresolved reference");
    assert_eq!(unresolved.dataset, None);
    assert_eq!(unresolved.layer, None);
    assert!(unresolved.resolution_diagnostic.is_some());
}
