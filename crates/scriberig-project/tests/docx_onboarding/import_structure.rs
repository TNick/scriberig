//! Verifies persisted DOCX source becomes a bounded semantic document tree.

use std::fs;
use std::sync::Arc;

use scriberig_project::{DocumentBlockView, ProjectService};
use scriberig_store_sqlite::SqliteStore;

use super::docx_project;

/// Scans and imports DOCX paragraph/table semantics from persisted source bytes.
#[test]
fn docx_onboarding_creates_document_paragraph_and_table_cell_entities() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    assert_eq!(scan.files[0].adapter, "docx");
    let project = scan.project.id;
    let expected_bytes = fs::read(root.join("report.docx")).expect("source bytes");
    let files = store
        .project_file_descriptors(project)
        .expect("scan manifest");
    assert_eq!(files[0].digest, *blake3::hash(&expected_bytes).as_bytes());
    let imported = service.import(project).expect("import DOCX");
    let entities = store.project_entities(project).expect("semantic records");
    let document = entities
        .iter()
        .find(|entity| entity.kind == "document.root")
        .expect("document root");
    let catalog = service
        .symbols_at(project, imported.snapshot)
        .expect("snapshot project catalog");
    assert!(
        catalog
            .iter()
            .any(|symbol| symbol.entity == document.entity)
    );
    let tree = service
        .document_tree(project, imported.snapshot, document.entity)
        .expect("snapshot-bound document tree");
    assert_eq!(
        tree.artifact_capability,
        Some(scriberig_project::ArtifactCapability::EditableWithOpaquePreservation)
    );
    assert_eq!(tree.snapshot, imported.snapshot);
    assert_eq!(tree.sections.len(), 1);
    assert!(matches!(
        &tree.sections[0].blocks[0],
        DocumentBlockView::Paragraph { paragraph }
            if paragraph.runs[0].text == "Existing site area"
    ));
    assert!(matches!(
        &tree.sections[0].blocks[2],
        DocumentBlockView::Table { table }
            if table.rows[0].cells[0].blocks.len() == 1
    ));
    let search = service
        .find_document_paragraphs(project, imported.snapshot, document.entity, "SITE", 0, 1)
        .expect("case-insensitive paragraph search");
    assert_eq!(search.total, 1);
    assert_eq!(search.matches[0].text, "Existing site area");
    let first_page = service
        .find_document_paragraphs(project, imported.snapshot, document.entity, "a", 0, 2)
        .expect("bounded paragraph search");
    assert_eq!(first_page.total, 3);
    assert!(first_page.has_more);
    let last_page = service
        .find_document_paragraphs(project, imported.snapshot, document.entity, "a", 2, 2)
        .expect("paragraph search continuation");
    assert_eq!(last_page.matches[0].text, "parcel 124");
    assert!(!last_page.has_more);
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "document.root")
            .count(),
        1
    );
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "document.paragraph")
            .count(),
        3
    );
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "document.table_cell")
            .count(),
        1
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
