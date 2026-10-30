//! Verifies spreadsheet wire requests and typed cell pages serialize stably.

use scriberig_protocol::{
    Request, Response, SpreadsheetCandidateValidation, SpreadsheetCellEdit,
    SpreadsheetCellFindRequest, SpreadsheetCellFindResponse, SpreadsheetCellMatch,
    SpreadsheetCellValue, SpreadsheetCellView, SpreadsheetCellsRequest, SpreadsheetCellsResponse,
    SpreadsheetEditRequest, SpreadsheetEditResponse, SpreadsheetRange, SpreadsheetRangeEditRequest,
    SpreadsheetRenameRequest,
};

/// Round trips exact snapshots, ranges, coordinates, and formula semantics.
#[test]
fn spreadsheet_page_contract_round_trips_typed_values() {
    let request = Request::SpreadsheetCells(SpreadsheetCellsRequest {
        project: "prj_01010101010101010101010101010101".into(),
        snapshot: "snp_0202020202020202020202020202020202020202020202020202020202020202".into(),
        worksheet: "ent_03030303030303030303030303030303".into(),
        range: Some(SpreadsheetRange {
            first_row: 3,
            first_column: 2,
            last_row: 8,
            last_column: 5,
        }),
        offset: 1,
        limit: 25,
    });
    let encoded = serde_json::to_string(&request).expect("encode request");
    assert_eq!(
        serde_json::from_str::<Request>(&encoded).expect("decode request"),
        request
    );
    let response = Response::SpreadsheetCells(SpreadsheetCellsResponse {
        project: "project".into(),
        snapshot: "snapshot".into(),
        worksheet: "worksheet".into(),
        artifact_capability: Some(scriberig_protocol::ArtifactEditCapability::LosslessEditable),
        artifact_diagnostic: None,
        cells: vec![SpreadsheetCellView {
            id: "cell".into(),
            row: 3,
            column: 2,
            value: SpreadsheetCellValue::Formula {
                expression: "=A3*3".into(),
                cached_value: None,
            },
            style: None,
        }],
        total: 2,
        has_more: true,
    });
    let encoded = serde_json::to_string(&response).expect("encode response");
    assert_eq!(
        serde_json::from_str::<Response>(&encoded).expect("decode response"),
        response
    );
}

/// Verifies spreadsheet edits expose candidate handles without source bytes.
#[test]
fn spreadsheet_edit_contract_round_trips_candidate_metadata() {
    let request = Request::SpreadsheetEdit(SpreadsheetEditRequest {
        project: "project-1".into(),
        snapshot: "snapshot-1".into(),
        worksheet: "worksheet-1".into(),
        row: 3,
        column: 2,
        value: SpreadsheetCellValue::Formula {
            expression: "=A3*4".into(),
            cached_value: None,
        },
    });
    let encoded = serde_json::to_value(&request).expect("encode edit request");
    let request_body = encoded["SpreadsheetEdit"]
        .as_object()
        .expect("spreadsheet edit request body");
    assert_eq!(
        request_body.len(),
        6,
        "cell edits send only scalar target/value fields"
    );
    assert!(!request_body.contains_key("cells"));
    assert!(!request_body.contains_key("workbook"));
    assert!(!request_body.contains_key("source_bytes"));
    assert_eq!(
        encoded["SpreadsheetEdit"]["value"],
        serde_json::json!({ "kind": "formula", "value": { "expression": "=A3*4", "cachedValue": null } })
    );
    let decoded: Request = serde_json::from_value(encoded).expect("decode edit request");
    assert_eq!(decoded, request);
    let response = Response::SpreadsheetEdit(SpreadsheetEditResponse {
        project: "project-1".into(),
        candidate_id: "candidate-1".into(),
        base_snapshot: "snapshot-1".into(),
        path: "budget.xlsx".into(),
        validation: SpreadsheetCandidateValidation::Pending,
    });
    let encoded = serde_json::to_value(&response).expect("encode candidate response");
    assert!(encoded["SpreadsheetEdit"].get("old").is_none());
    assert!(encoded["SpreadsheetEdit"].get("new").is_none());
    let decoded: Response = serde_json::from_value(encoded).expect("decode candidate response");
    assert_eq!(decoded, response);
}

/// Round trips a typed bounded range edit without changing per-cell value fidelity.
#[test]
fn spreadsheet_range_edit_contract_round_trips_typed_updates() {
    let request = Request::SpreadsheetRangeEdit(SpreadsheetRangeEditRequest {
        project: "project-1".into(),
        snapshot: "snapshot-1".into(),
        worksheet: "worksheet-1".into(),
        edits: vec![
            SpreadsheetCellEdit {
                row: 3,
                column: 1,
                value: SpreadsheetCellValue::Number("12.5".into()),
            },
            SpreadsheetCellEdit {
                row: 4,
                column: 2,
                value: SpreadsheetCellValue::Formula {
                    expression: "=A3*2".into(),
                    cached_value: None,
                },
            },
        ],
    });
    let encoded = serde_json::to_string(&request).expect("encode request");
    assert_eq!(
        serde_json::from_str::<Request>(&encoded).expect("decode request"),
        request
    );
}

/// Round trips a worksheet rename request and candidate response without source bytes.
#[test]
fn spreadsheet_rename_contract_round_trips_candidate_metadata() {
    let request = Request::SpreadsheetRename(SpreadsheetRenameRequest {
        project: "project-1".into(),
        snapshot: "snapshot-1".into(),
        worksheet: "worksheet-1".into(),
        new_name: "Plan".into(),
    });
    let encoded = serde_json::to_value(&request).expect("encode rename request");
    assert_eq!(encoded["SpreadsheetRename"]["new_name"], "Plan");
    assert_eq!(
        serde_json::from_value::<Request>(encoded).expect("decode rename request"),
        request
    );
    let response = Response::SpreadsheetRename(SpreadsheetEditResponse {
        project: "project-1".into(),
        candidate_id: "candidate-1".into(),
        base_snapshot: "snapshot-1".into(),
        path: "budget.xlsx".into(),
        validation: SpreadsheetCandidateValidation::Pending,
    });
    let encoded = serde_json::to_value(&response).expect("encode rename candidate");
    assert!(encoded["SpreadsheetRename"].get("old").is_none());
    assert!(encoded["SpreadsheetRename"].get("new").is_none());
    assert_eq!(
        serde_json::from_value::<Response>(encoded).expect("decode rename candidate"),
        response
    );
}

/// Round trips bounded exact-snapshot spreadsheet search handles and paging.
#[test]
fn spreadsheet_find_contract_round_trips_semantic_cell_matches() {
    let request = Request::SpreadsheetCellFind(SpreadsheetCellFindRequest {
        project: "project-1".into(),
        snapshot: "snapshot-2".into(),
        workbook: "workbook-3".into(),
        query: "budget".into(),
        offset: 2,
        limit: 10,
    });
    let encoded = serde_json::to_string(&request).expect("encode find request");
    assert_eq!(
        serde_json::from_str::<Request>(&encoded).expect("decode find request"),
        request
    );
    let response = Response::SpreadsheetCellFind(SpreadsheetCellFindResponse {
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
        total: 3,
        has_more: false,
    });
    let encoded = serde_json::to_string(&response).expect("encode find response");
    assert_eq!(
        serde_json::from_str::<Response>(&encoded).expect("decode find response"),
        response
    );
}
