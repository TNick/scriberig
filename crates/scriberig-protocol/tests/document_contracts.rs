//! Verifies document edit candidate wire contracts round-trip without source bytes.

use scriberig_protocol::{
    DocumentBlockView, DocumentCandidateValidation, DocumentInspectRequest,
    DocumentInspectResponse, DocumentParagraphEditRequest, DocumentParagraphEditResponse,
    DocumentParagraphView, DocumentSectionView, DocumentStyleView, DocumentTableCellEditRequest,
    Request, Response,
};

/// Preserves exact semantic target and candidate identity across serialization.
#[test]
fn document_paragraph_edit_contract_round_trips_candidate_metadata() {
    let request = Request::DocumentParagraphEdit(DocumentParagraphEditRequest {
        project: "project-1".into(),
        snapshot: "snapshot-1".into(),
        paragraph: "paragraph-1".into(),
        replacement: "Proposed site area".into(),
    });
    let encoded = serde_json::to_string(&request).expect("encode request");
    assert_eq!(
        serde_json::from_str::<Request>(&encoded).expect("decode request"),
        request
    );
    let response = Response::DocumentParagraphEdit(DocumentParagraphEditResponse {
        project: "project-1".into(),
        candidate_id: "candidate-1".into(),
        base_snapshot: "snapshot-1".into(),
        path: "report.docx".into(),
        validation: DocumentCandidateValidation::Pending,
    });
    let encoded = serde_json::to_value(&response).expect("encode response");
    assert!(encoded["DocumentParagraphEdit"].get("old").is_none());
    assert!(encoded["DocumentParagraphEdit"].get("new").is_none());
    assert_eq!(
        serde_json::from_value::<Response>(encoded).expect("decode response"),
        response
    );
}

/// Round trips the stable cell target without adding package bytes to the request.
#[test]
fn document_table_cell_edit_contract_round_trips_cell_identity() {
    let request = Request::DocumentTableCellEdit(DocumentTableCellEditRequest {
        project: "project-1".into(),
        snapshot: "snapshot-2".into(),
        cell: "cell-3".into(),
        replacement: "parcel 126".into(),
    });
    let encoded = serde_json::to_string(&request).expect("encode request");
    assert_eq!(
        serde_json::from_str::<Request>(&encoded).expect("decode request"),
        request
    );
}

/// Preserves document tree order and stable paragraph identity on the wire.
#[test]
fn document_inspect_contract_round_trips_typed_paragraph_views() {
    let request = Request::DocumentInspect(DocumentInspectRequest {
        project: "project-1".into(),
        snapshot: "snapshot-1".into(),
        document: "document-1".into(),
    });
    let encoded = serde_json::to_string(&request).expect("encode request");
    assert_eq!(
        serde_json::from_str::<Request>(&encoded).expect("decode request"),
        request
    );
    let response = Response::DocumentInspect(DocumentInspectResponse {
        project: "project-1".into(),
        snapshot: "snapshot-1".into(),
        document: "document-1".into(),
        artifact_capability: Some(
            scriberig_protocol::ArtifactEditCapability::EditableWithOpaquePreservation,
        ),
        styles: vec![DocumentStyleView {
            key: "BodyText".into(),
            name: Some("Body Text".into()),
        }],
        sections: vec![DocumentSectionView {
            id: "section-1".into(),
            blocks: vec![DocumentBlockView::Paragraph {
                paragraph: DocumentParagraphView {
                    id: "paragraph-1".into(),
                    runs: Vec::new(),
                    style: None,
                },
            }],
        }],
    });
    let encoded = serde_json::to_value(&response).expect("encode response");
    assert_eq!(
        serde_json::from_value::<Response>(encoded).expect("decode response"),
        response
    );
}
