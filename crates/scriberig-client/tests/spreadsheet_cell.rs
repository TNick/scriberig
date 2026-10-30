//! Verifies the client preserves exact cell coordinates and sparse blank results.

use scriberig_client::Client;
use scriberig_protocol::{
    ProtocolTransport, ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
    SpreadsheetCellInspectResponse,
};

/// Confirms a one-based cell selector and returns an absent blank cell.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Validates the coordinate request and returns the blank cell response.
    ///
    /// # Parameters
    /// - `request`: Cell inspection operation envelope.
    ///
    /// # Returns
    /// Typed response for the requested coordinate.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(
            matches!(request.request, Request::SpreadsheetCellInspect(ref request)
            if request.project == "project-1"
                && request.snapshot == "snapshot-2"
                && request.worksheet == "sheet-3"
                && request.row == 4
                && request.column == 5)
        );
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::SpreadsheetCellInspect(SpreadsheetCellInspectResponse {
                project: "project-1".into(),
                snapshot: "snapshot-2".into(),
                worksheet: "sheet-3".into(),
                row: 4,
                column: 5,
                cell: None,
            }),
        }
    }
}

/// Returns the typed sparse blank result from the semantic client.
#[test]
fn client_returns_explicit_sparse_blank_cell_inspection() {
    let result = Client::new(FixtureTransport)
        .inspect_spreadsheet_cell("project-1", "snapshot-2", "sheet-3", 4, 5)
        .expect("cell inspection");
    assert_eq!(result.row, 4);
    assert_eq!(result.column, 5);
    assert_eq!(result.cell, None);
}
