//! Verifies the client preserves snapshot-bound spreadsheet search identities.

use scriberig_client::Client;
use scriberig_protocol::{
    ProtocolTransport, ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
    SpreadsheetCellFindResponse, SpreadsheetCellMatch,
};

/// Validates exact workbook search selectors and returns one semantic match.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Confirms the exact search request and returns a bounded match page.
    ///
    /// # Parameters
    /// - `request`: Protocol envelope sent by the client.
    ///
    /// # Returns
    /// Search response for one exact semantic cell handle.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(matches!(
            request.request,
            Request::SpreadsheetCellFind(ref request)
                if request.project == "project-1"
                    && request.snapshot == "snapshot-2"
                    && request.workbook == "workbook-3"
                    && request.query == "budget"
                    && request.offset == 0
                    && request.limit == 10
        ));
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::SpreadsheetCellFind(SpreadsheetCellFindResponse {
                project: "project-1".into(),
                snapshot: "snapshot-2".into(),
                workbook: "workbook-3".into(),
                matches: vec![SpreadsheetCellMatch {
                    worksheet: "worksheet-4".into(),
                    cell: "cell-5".into(),
                    row: 12,
                    column: 3,
                    authored_value: "Budget total".into(),
                }],
                total: 1,
                has_more: false,
            }),
        }
    }
}

/// Returns stable spreadsheet search handles through the reusable client.
#[test]
fn client_returns_snapshot_bound_spreadsheet_matches() {
    let response = Client::new(FixtureTransport)
        .find_spreadsheet_cells("project-1", "snapshot-2", "workbook-3", "budget", 0, 10)
        .expect("spreadsheet search");
    assert_eq!(response.snapshot, "snapshot-2");
    assert_eq!(response.matches[0].worksheet, "worksheet-4");
    assert_eq!(response.matches[0].cell, "cell-5");
    assert_eq!(response.matches[0].row, 12);
}
