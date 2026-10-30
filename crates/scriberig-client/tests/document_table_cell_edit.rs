//! Verifies exact-base DOCX table-cell candidates over the typed client.

use scriberig_client::Client;
use scriberig_protocol::{
    DocumentCandidateValidation, DocumentParagraphEditResponse, ProtocolTransport, ProtocolVersion,
    Request, RequestEnvelope, Response, ResponseEnvelope,
};

/// Confirms cell edits carry only stable handles and authored replacement text.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Validates one cell-target request and returns candidate metadata.
    ///
    /// # Parameters
    /// - `request`: Table-cell edit operation envelope.
    ///
    /// # Returns
    /// Typed candidate response envelope.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(
            matches!(request.request, Request::DocumentTableCellEdit(ref edit)
            if edit.project == "project-1"
                && edit.snapshot == "snapshot-2"
                && edit.cell == "cell-3"
                && edit.replacement == "parcel 126")
        );
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::DocumentTableCellEdit(DocumentParagraphEditResponse {
                project: "project-1".into(),
                candidate_id: "candidate-4".into(),
                base_snapshot: "snapshot-2".into(),
                path: "report.docx".into(),
                validation: DocumentCandidateValidation::Pending,
            }),
        }
    }
}

/// Returns the reviewable candidate from a semantic cell operation.
#[test]
fn client_proposes_table_cell_candidate_by_cell_identity() {
    let candidate = Client::new(FixtureTransport)
        .propose_document_table_cell_edit("project-1", "snapshot-2", "cell-3", "parcel 126")
        .expect("table-cell candidate");
    assert_eq!(candidate.candidate_id, "candidate-4");
    assert_eq!(candidate.base_snapshot, "snapshot-2");
    assert_eq!(candidate.path, "report.docx");
}
