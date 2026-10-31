//! Verifies paragraph insertion remains an exact-base semantic client operation.

use scriberig_client::Client;
use scriberig_protocol::{
    DocumentCandidateValidation, DocumentParagraphInsertResponse, ProtocolTransport,
    ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
};

/// Validates an exact section insertion request and returns a candidate handle.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Confirms the insertion selector and returns deterministic candidate metadata.
    ///
    /// # Parameters
    /// - `request`: Paragraph insertion operation envelope.
    ///
    /// # Returns
    /// Typed candidate response envelope.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(
            matches!(request.request, Request::DocumentParagraphInsert(ref request)
            if request.project == "project-1"
                && request.snapshot == "snapshot-2"
                && request.section == "section-3"
                && request.text == "Appended text")
        );
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::DocumentParagraphInsert(DocumentParagraphInsertResponse {
                project: "project-1".into(),
                candidate_id: "candidate-4".into(),
                base_snapshot: "snapshot-2".into(),
                path: "report.docx".into(),
                validation: DocumentCandidateValidation::Pending,
            }),
        }
    }
}

/// Returns the exact-base section candidate from the typed remote client method.
#[test]
fn client_returns_document_paragraph_insertion_candidate() {
    let candidate = Client::new(FixtureTransport)
        .propose_document_paragraph_insert("project-1", "snapshot-2", "section-3", "Appended text")
        .expect("paragraph insertion");
    assert_eq!(candidate.candidate_id, "candidate-4");
    assert_eq!(candidate.base_snapshot, "snapshot-2");
    assert_eq!(candidate.path, "report.docx");
}
