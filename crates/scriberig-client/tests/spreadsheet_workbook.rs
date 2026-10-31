//! Verifies the reusable client preserves exact-snapshot workbook summary fields.

use scriberig_client::Client;
use scriberig_protocol::{
    ProtocolTransport, ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
    SpreadsheetRange, SpreadsheetTableSummary, SpreadsheetWorkbookResponse,
    SpreadsheetWorksheetSummary,
};

/// Confirms the workbook request and returns one typed worksheet summary.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Validates the exact workbook selector and returns deterministic metadata.
    ///
    /// # Parameters
    /// - `request`: Workbook operation envelope from the client.
    ///
    /// # Returns
    /// A typed workbook response envelope.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(
            matches!(request.request, Request::SpreadsheetWorkbook(ref request)
            if request.project == "project-1"
                && request.snapshot == "snapshot-2"
                && request.workbook == "workbook-3")
        );
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::SpreadsheetWorkbook(SpreadsheetWorkbookResponse {
                project: "project-1".into(),
                snapshot: "snapshot-2".into(),
                workbook: "workbook-3".into(),
                artifact_capability: None,
                worksheets: vec![SpreadsheetWorksheetSummary {
                    id: "sheet-4".into(),
                    name: "Budget".into(),
                    used_range: Some(SpreadsheetRange {
                        first_row: 1,
                        first_column: 1,
                        last_row: 3,
                        last_column: 4,
                    }),
                    tables: vec![SpreadsheetTableSummary {
                        id: "table-5".into(),
                        name: "BudgetTable".into(),
                        range: SpreadsheetRange {
                            first_row: 1,
                            first_column: 1,
                            last_row: 3,
                            last_column: 4,
                        },
                    }],
                }],
            }),
        }
    }
}

/// Returns the exact-snapshot workbook and worksheet summary from the client.
#[test]
fn client_returns_workbook_summary_and_used_range() {
    let summary = Client::new(FixtureTransport)
        .inspect_spreadsheet_workbook("project-1", "snapshot-2", "workbook-3")
        .expect("workbook summary");
    assert_eq!(summary.snapshot, "snapshot-2");
    assert_eq!(summary.worksheets[0].name, "Budget");
    assert_eq!(summary.worksheets[0].tables[0].name, "BudgetTable");
    assert_eq!(
        summary.worksheets[0]
            .used_range
            .as_ref()
            .unwrap()
            .last_column,
        4
    );
}
