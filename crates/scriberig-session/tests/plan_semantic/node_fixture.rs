//! Builds shared plan-node fixtures for semantic plan tests.

use scriberig_core::EntityId;
use scriberig_session::{PlanNode, PlanNodeParent, PlanNodeStatus};

/// Creates a valid unstarted plan node with a stable identity.
///
/// # Parameters
/// - `id`: Stable node identity byte repeated across the ID.
/// - `branch`: Existing branch used as the structural parent.
/// - `title`: Human-readable node title and purpose.
///
/// # Returns
/// A planned node with no execution attempt.
pub(super) fn node(id: u8, branch: EntityId, title: &str) -> PlanNode {
    PlanNode {
        id: EntityId::from_bytes([id; 16]),
        parent: Some(PlanNodeParent::Branch(branch)),
        title: title.to_owned(),
        purpose: title.to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}
