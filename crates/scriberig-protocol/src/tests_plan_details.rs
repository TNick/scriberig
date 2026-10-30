//! Verifies exact-snapshot plan detail transport types.

use super::*;

/// Round-trips bounded gate and evidence detail requests and responses.
#[test]
fn plan_node_detail_request_and_page_round_trip() {
    let session = SessionReference {
        id: "session-a".to_owned(),
    };
    round_trip(&SessionPlanNodeDetailsRequest {
        session,
        plan: "plan-1".to_owned(),
        node: "node-a".to_owned(),
        snapshot: "snapshot-1".to_owned(),
        offset: 2,
        limit: 16,
    });
    round_trip(&SessionPlanNodeDetailsPage {
        offset: 2,
        gates: Vec::new(),
        evidence: Vec::new(),
    });
}
