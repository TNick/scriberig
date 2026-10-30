//! Verifies remote document inspection preserves exact semantic scope.

use super::*;

/// Returns a document tree only when the remote identities match the request.
#[test]
fn remote_document_read_preserves_exact_snapshot_and_root() {
    let project = scriberig_core::ProjectId::from_bytes([121; 16]);
    let snapshot = SnapshotId::new([122; 32]);
    let document = EntityId::from_bytes([123; 16]);
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "document-read".to_owned(),
            response: Response::DocumentInspect(scriberig_protocol::DocumentInspectResponse {
                project: project.to_string(),
                snapshot: snapshot.to_string(),
                document: document.to_string(),
                artifact_capability: Some(
                    scriberig_protocol::ArtifactEditCapability::LosslessEditable,
                ),
                styles: Vec::new(),
                sections: Vec::new(),
            }),
        },
    }));

    let tree = adapter
        .inspect_document(
            &ApplicationContext::default(),
            &scriberig_app::DocumentTreeQuery {
                project,
                snapshot,
                document,
            },
        )
        .expect("remote exact-snapshot document read");

    assert_eq!(tree.project, project.to_string());
    assert_eq!(tree.snapshot, snapshot.to_string());
    assert_eq!(tree.document, document.to_string());
    assert_eq!(
        tree.artifact_capability,
        Some(scriberig_protocol::ArtifactEditCapability::LosslessEditable)
    );
}

/// Preserves the exact-base identity for a remote document paragraph candidate.
#[test]
fn remote_document_edit_preserves_candidate_and_validation() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/document-paragraph-edit.json"
    ))
    .expect("shared web/TUI paragraph edit fixture");
    let project = fixture["project"]
        .as_str()
        .expect("fixture project")
        .parse::<scriberig_core::ProjectId>()
        .expect("valid project identity");
    let snapshot = fixture["snapshot"]
        .as_str()
        .expect("fixture snapshot")
        .parse::<SnapshotId>()
        .expect("valid snapshot identity");
    let paragraph = fixture["paragraph"]
        .as_str()
        .expect("fixture paragraph")
        .parse::<EntityId>()
        .expect("valid paragraph identity");
    let candidate_id = fixture["candidateId"]
        .as_str()
        .expect("fixture candidate ID");
    let path = fixture["path"].as_str().expect("fixture artifact path");
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "document-edit".to_owned(),
            response: Response::DocumentParagraphEdit(
                scriberig_protocol::DocumentParagraphEditResponse {
                    project: project.to_string(),
                    candidate_id: candidate_id.to_owned(),
                    base_snapshot: snapshot.to_string(),
                    path: path.to_owned(),
                    validation: scriberig_protocol::DocumentCandidateValidation::Pending,
                },
            ),
        },
    }));

    let candidate = adapter
        .propose_document_paragraph_edit(
            &ApplicationContext::default(),
            &scriberig_app::DocumentParagraphEditQuery {
                project,
                snapshot,
                paragraph,
                replacement: fixture["replacement"]
                    .as_str()
                    .expect("fixture replacement")
                    .to_owned(),
            },
        )
        .expect("remote document candidate");

    assert_eq!(candidate.id, candidate_id);
    assert_eq!(candidate.project, project);
    assert_eq!(candidate.base_snapshot, snapshot);
    assert_eq!(candidate.path, path);
    assert_eq!(
        candidate.validation,
        scriberig_project::CandidateValidation::Pending
    );
}

/// Preserves the exact-base identity for a remote table-cell candidate.
#[test]
fn remote_document_cell_edit_preserves_candidate_and_validation() {
    let project = scriberig_core::ProjectId::from_bytes([131; 16]);
    let snapshot = SnapshotId::new([132; 32]);
    let cell = EntityId::from_bytes([133; 16]);
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "document-cell-edit".to_owned(),
            response: Response::DocumentTableCellEdit(
                scriberig_protocol::DocumentParagraphEditResponse {
                    project: project.to_string(),
                    candidate_id: "doc-cell-candidate-1".to_owned(),
                    base_snapshot: snapshot.to_string(),
                    path: "report.docx".to_owned(),
                    validation: scriberig_protocol::DocumentCandidateValidation::Pending,
                },
            ),
        },
    }));
    let candidate = adapter
        .propose_document_table_cell_edit(
            &ApplicationContext::default(),
            &scriberig_app::DocumentTableCellEditQuery {
                project,
                snapshot,
                cell,
                replacement: "parcel 125".to_owned(),
            },
        )
        .expect("remote cell candidate");
    assert_eq!(candidate.id, "doc-cell-candidate-1");
    assert_eq!(candidate.project, project);
    assert_eq!(candidate.base_snapshot, snapshot);
    assert_eq!(candidate.path, "report.docx");
    assert_eq!(
        candidate.validation,
        scriberig_project::CandidateValidation::Pending
    );
}
