//! Verifies DOCX table cells can be edited through their stable cell handles.

use std::fs;
use std::io::Read;
use std::sync::Arc;

use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store_sqlite::SqliteStore;
use zip::ZipArchive;

use crate::docx_project;

/// Creates, validates, applies, and reopens a cell-targeted DOCX candidate.
#[test]
fn docx_table_cell_handle_edits_one_cell_as_a_durable_candidate() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service
        .scan(None, &root)
        .expect("scan DOCX project")
        .project
        .id;
    let base = service.import(project).expect("import DOCX").snapshot;
    let original = fs::read(root.join("report.docx")).expect("read original package");
    let cell = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "document.table_cell")
        .expect("table cell")
        .entity;

    let candidate = service
        .propose_docx_table_cell_edit(project, base, cell, "parcel 126")
        .expect("propose cell edit");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("document.update_table_cell")
    );
    assert_eq!(candidate.semantic_target, Some(cell));
    assert_eq!(candidate.semantic_changes.len(), 1);
    assert_eq!(candidate.semantic_changes[0].target, cell);
    assert_eq!(candidate.semantic_changes[0].property, "table_cell.text");
    assert_eq!(candidate.semantic_changes[0].before, "parcel 124");
    assert_eq!(candidate.semantic_changes[0].after, "parcel 126");
    assert_eq!(
        service
            .candidate(&candidate.id)
            .expect("reload candidate")
            .expect("durable candidate")
            .semantic_changes,
        candidate.semantic_changes
    );
    assert_eq!(
        fs::read(root.join("report.docx")).expect("live package"),
        original
    );
    assert_eq!(
        store
            .project(project)
            .expect("project state")
            .expect("project")
            .current_snapshot,
        Some(base)
    );

    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate cell candidate");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("apply cell candidate");
    assert_ne!(applied.successor_snapshot, base);
    let materialized = fs::read(root.join("report.docx")).expect("materialized package");
    let mut archive =
        ZipArchive::new(std::io::Cursor::new(materialized)).expect("reopen materialized DOCX");
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("main document part")
        .read_to_string(&mut xml)
        .expect("read main document part");
    assert!(xml.contains("parcel 126"));
    assert!(xml.contains("Preserve this paragraph"));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
