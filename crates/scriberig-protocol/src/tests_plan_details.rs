//! Verifies bounded plan detail transport types.

use super::*;

/// Round-trips bounded gate and evidence detail requests and responses.
#[test]
fn plan_details_request_and_response_round_trip() {
    let session = SessionReference {
        id: "session-a".to_owned(),
    };
    round_trip(&SessionPlanDetailsRequest {
        session,
        plan: "plan-1".to_owned(),
        expected_revision: 3,
        snapshot: Some("snapshot-1".to_owned()),
        gate_offset: 2,
        evidence_offset: 4,
        limit: 16,
    });
    round_trip(&SessionPlanDetailsResponse {
        plan: "plan-1".to_owned(),
        revision: 3,
        snapshot: Some("snapshot-1".to_owned()),
        gate_total: 2,
        gate_offset: 2,
        gates: Vec::new(),
        gate_next_offset: None,
        evidence_total: 4,
        evidence_offset: 4,
        evidence: Vec::new(),
        evidence_next_offset: None,
    });
}
