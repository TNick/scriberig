//! Verifies DOCX paragraph operations reject entities of the wrong semantic kind.

use std::fs;
use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

use crate::docx_project;

/// Rejects a document-root identity as a paragraph edit target without mutation.
#[test]
fn docx_paragraph_operation_rejects_document_root_target() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let base = service.import(project).expect("import DOCX").snapshot;
    let document = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "document.root")
        .expect("document root");
    let original = fs::read(root.join("report.docx")).expect("original source");

    assert!(
        service
            .propose_docx_paragraph_edit(project, base, document.entity, "not a paragraph")
            .is_err()
    );
    assert_eq!(
        store
            .project(project)
            .expect("project state")
            .expect("project")
            .current_snapshot,
        Some(base)
    );
    assert_eq!(
        fs::read(root.join("report.docx")).expect("source after rejection"),
        original
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
