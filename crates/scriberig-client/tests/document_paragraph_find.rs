//! Verifies the client keeps paragraph searches bound to their snapshot and document.

use scriberig_client::Client;
use scriberig_protocol::{
    DocumentParagraphFindResponse, DocumentParagraphMatch, ProtocolTransport, ProtocolVersion,
    Request, RequestEnvelope, Response, ResponseEnvelope,
};

/// Confirms the exact document search request and returns one semantic match.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Validates the search selector and returns deterministic paragraph metadata.
    ///
    /// # Parameters
    /// - `request`: Paragraph search operation envelope.
    ///
    /// # Returns
    /// Typed paragraph search response envelope.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(
            matches!(request.request, Request::DocumentParagraphFind(ref request)
            if request.project == "project-1"
                && request.snapshot == "snapshot-2"
                && request.document == "document-3"
                && request.query == "site"
                && request.offset == 0
                && request.limit == 10)
        );
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::DocumentParagraphFind(DocumentParagraphFindResponse {
                project: "project-1".into(),
                snapshot: "snapshot-2".into(),
                document: "document-3".into(),
                matches: vec![DocumentParagraphMatch {
                    paragraph: "paragraph-4".into(),
                    section: "section-5".into(),
                    text: "Existing site area".into(),
                }],
                total: 1,
                has_more: false,
            }),
        }
    }
}

/// Returns a stable paragraph match from the typed remote client method.
#[test]
fn client_returns_snapshot_bound_document_paragraph_matches() {
    let results = Client::new(FixtureTransport)
        .find_document_paragraphs("project-1", "snapshot-2", "document-3", "site", 0, 10)
        .expect("paragraph search");
    assert_eq!(results.snapshot, "snapshot-2");
    assert_eq!(results.matches[0].paragraph, "paragraph-4");
    assert_eq!(results.matches[0].text, "Existing site area");
}
