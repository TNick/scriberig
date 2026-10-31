//! Verifies exact-snapshot gate and evidence detail pages.

use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanEvidenceOwner, PlanSemanticSnapshot};

/// Verifies node-owned and gate-owned evidence appear in stable pages.
///
/// # Parameters
/// - `snapshot`: Encoded semantic plan records.
/// - `plan`: Source plan used to verify gate projection.
/// - `node_id`: Stable node receiving the gate and evidence.
/// - `gate_id`: Stable gate owned by the node.
pub(super) fn assert_node_detail_pages(
    snapshot: &PlanSemanticSnapshot,
    plan: &Plan,
    node_id: EntityId,
    gate_id: EntityId,
) {
    let detail_page = snapshot
        .node_details(node_id, 0, 1)
        .expect("first detail page");
    assert_eq!(detail_page.gates, vec![plan.gates[0].clone()]);
    assert_eq!(detail_page.evidence.len(), 1);
    assert_eq!(
        detail_page.evidence[0].owner,
        PlanEvidenceOwner::Gate(gate_id)
    );
    let next_page = snapshot
        .node_details(node_id, 1, 1)
        .expect("next detail page");
    assert_eq!(next_page.gates, Vec::<scriberig_session::PlanGate>::new());
    assert_eq!(next_page.evidence.len(), 1);
    assert_eq!(
        next_page.evidence[0].owner,
        PlanEvidenceOwner::Node(node_id)
    );
}
