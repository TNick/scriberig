//! Verifies local HTTP document edits receive host-owned project Propose grants.

use std::io::Write;
use std::sync::Arc;

use super::{HttpState, dispatch};
use crate::development::ensure_fixture;
use scriberig_app::{ApplicationService, StoreInspectionBackend};
use scriberig_core::EntityId;
use scriberig_protocol::{
    DocumentCandidateValidation, DocumentInspectRequest, DocumentParagraphEditRequest,
    DocumentParagraphFindRequest, DocumentParagraphInsertRequest, DocumentTableCellEditRequest,
    Response, ResponseEnvelope,
};
use scriberig_store::{RevisionStore, SnapshotStore};
use zip::write::SimpleFileOptions;

#[allow(
    clippy::too_many_lines,
    reason = "one end-to-end scenario checks HTTP authorization, read, search, candidate, and no-live-write behavior"
)]
/// Creates a paragraph candidate from the local HTTP endpoint without live writes.
#[test]
fn http_document_edit_resolves_local_project_propose_grant_from_catalog() {
    let root = std::env::temp_dir().join(format!("scriberig-http-docx-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Existing site area</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>parcel 124</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#)
        .expect("document XML");
    let original = archive.finish().expect("finish DOCX").into_inner();
    let path = root.join("report.docx");
    std::fs::write(&path, &original).expect("write DOCX");
    let database = root.with_extension("sqlite");
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(&database).expect("database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let imported = projects.import(scan.project.id).expect("import DOCX");
    let paragraph = store
        .project_entities(scan.project.id)
        .expect("entity ownership")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store
                .resolve(imported.snapshot, entity.entity)
                .ok()
                .flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let payload: serde_json::Value = serde_json::from_slice(revision.payload()).ok()?;
            (payload["runs"][0]["text"] == "Existing site area").then_some(entity.entity)
        })
        .expect("semantic paragraph");
    let document = store
        .project_entities(scan.project.id)
        .expect("entity ownership")
        .into_iter()
        .find(|entity| entity.kind == "document.root")
        .expect("document root");
    let service = ApplicationService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects);
    let state = HttpState {
        service,
        principal: scriberig_app::Principal::new("local-user"),
        fixture: ensure_fixture(&store).expect("development fixture"),
        allowed_origin: None,
        project_store: Some(Arc::clone(&store)),
        allowed_project_roots: vec![root.clone()],
        plan_lifecycle_handler: None,
        agent_execution_handler: None,
        provider_readiness_handler: None,
    };
    let request = DocumentParagraphEditRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        paragraph: paragraph.to_string(),
        replacement: "Proposed site area".into(),
    };
    let body = serde_json::to_string(&request).expect("encode request");
    let response = dispatch(&state, "document_paragraph_edit", &body, "http-docx".into())
        .expect("dispatch document edit");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let candidate = match envelope.response {
        Response::DocumentParagraphEdit(candidate) => candidate,
        other => panic!("expected document candidate, got {other:?}"),
    };
    assert_eq!(candidate.path, "report.docx");
    assert_eq!(candidate.validation, DocumentCandidateValidation::Pending);
    assert_eq!(std::fs::read(&path).expect("live source"), original);
    let cell = store
        .project_entities(scan.project.id)
        .expect("cell ownership")
        .into_iter()
        .find(|entity| entity.kind == "document.table_cell")
        .expect("semantic table cell");
    let cell_request = DocumentTableCellEditRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        cell: cell.entity.to_string(),
        replacement: "parcel 125".into(),
    };
    let body = serde_json::to_string(&cell_request).expect("encode cell request");
    let response = dispatch(
        &state,
        "document_table_cell_edit",
        &body,
        "http-docx-cell".into(),
    )
    .expect("dispatch table-cell edit");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let cell_candidate = match envelope.response {
        Response::DocumentTableCellEdit(candidate) => candidate,
        other => panic!("expected table-cell candidate, got {other:?}"),
    };
    assert_eq!(cell_candidate.path, "report.docx");
    assert_eq!(
        cell_candidate.validation,
        DocumentCandidateValidation::Pending
    );
    assert_eq!(
        std::fs::read(&path).expect("live source after cell edit"),
        original
    );
    let section = store
        .project_entities(scan.project.id)
        .expect("entity ownership")
        .into_iter()
        .find(|entity| entity.kind == "document.section")
        .expect("document section");
    let insert_request = DocumentParagraphInsertRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        section: section.entity.to_string(),
        text: "Appended through HTTP".into(),
    };
    let body = serde_json::to_string(&insert_request).expect("encode insertion request");
    let response = dispatch(
        &state,
        "document_paragraph_insert",
        &body,
        "http-docx-insert".into(),
    )
    .expect("dispatch paragraph insertion");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let inserted = match envelope.response {
        Response::DocumentParagraphInsert(candidate) => candidate,
        other => panic!("expected insertion candidate, got {other:?}"),
    };
    assert_eq!(inserted.path, "report.docx");
    assert_eq!(inserted.validation, DocumentCandidateValidation::Pending);
    assert_eq!(
        std::fs::read(path).expect("live source remains unchanged"),
        original
    );

    let inspect_request = DocumentInspectRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        document: document.entity.to_string(),
    };
    let body = serde_json::to_string(&inspect_request).expect("encode inspect request");
    let response = dispatch(&state, "document_inspect", &body, "http-docx-read".into())
        .expect("dispatch document inspect");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let tree = match envelope.response {
        Response::DocumentInspect(tree) => tree,
        other => panic!("expected document tree, got {other:?}"),
    };
    assert_eq!(tree.snapshot, imported.snapshot.to_string());
    assert_eq!(tree.sections[0].blocks.len(), 2);

    let find_request = DocumentParagraphFindRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        document: document.entity.to_string(),
        query: "SITE".into(),
        offset: 0,
        limit: 5,
    };
    let body = serde_json::to_string(&find_request).expect("encode paragraph search");
    let response = dispatch(
        &state,
        "document_paragraph_find",
        &body,
        "http-docx-find".into(),
    )
    .expect("dispatch paragraph search");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let found = match envelope.response {
        Response::DocumentParagraphFind(found) => found,
        other => panic!("expected paragraph matches, got {other:?}"),
    };
    assert_eq!(found.matches.len(), 1);
    assert_eq!(found.matches[0].text, "Existing site area");

    drop(state);
    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}
