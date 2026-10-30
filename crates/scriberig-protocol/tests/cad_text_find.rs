//! Verifies bounded CAD text search protocol contracts round-trip safely.

use scriberig_protocol::{CadTextFindRequest, CadTextFindResponse, Request, Response};

/// Preserves exact drawing scope and pagination in the typed search contract.
#[test]
fn cad_text_find_request_and_response_round_trip() {
    let request = Request::CadTextFind(CadTextFindRequest {
        project: "project-1".into(),
        snapshot: "snapshot-2".into(),
        drawing: "drawing-3".into(),
        query: "parcel".into(),
        offset: 5,
        limit: 10,
    });
    let encoded = serde_json::to_string(&request).expect("encode search request");
    assert_eq!(
        serde_json::from_str::<Request>(&encoded).expect("decode request"),
        request
    );

    let response = Response::CadTextFind(CadTextFindResponse {
        project: "project-1".into(),
        snapshot: "snapshot-2".into(),
        drawing: "drawing-3".into(),
        matches: Vec::new(),
        total: 5,
        has_more: false,
    });
    let encoded = serde_json::to_string(&response).expect("encode search response");
    assert_eq!(
        serde_json::from_str::<Response>(&encoded).expect("decode response"),
        response
    );
}
