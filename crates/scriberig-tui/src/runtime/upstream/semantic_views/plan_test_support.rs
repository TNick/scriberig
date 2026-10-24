//! Shared durable plan builders for the plan projection tests.

use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanNode, PlanNodeParent};

/// Builds a deterministic plan with structural and dependency relations diverging.
///
/// - `plan_id`: Durable plan identity.
/// - `branch`: Structural parent branch identity.
/// - `first`: First durable node identity.
/// - `second`: Second durable node identity.
///
/// # Returns
/// A plan with one structural edge and a reverse dependency edge.
pub(super) fn sample_plan(
    plan_id: EntityId,
    branch: EntityId,
    first: EntityId,
    second: EntityId,
) -> Plan {
    Plan {
        id: plan_id,
        session: Some(EntityId::from_bytes([9; 16])),
        anchor: None,
        goal: "Verify graph projection".to_owned(),
        revision: 4,
        status: scriberig_session::PlanStatus::Running,
        root_branch: branch,
        nodes: vec![
            node(first, None),
            node(second, Some(PlanNodeParent::Node(first))),
        ],
        dependencies: vec![scriberig_session::PlanDependency {
            prerequisite: second,
            dependent: first,
            fulfilled_by: None,
        }],
        source_snapshot: None,
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        coordinator: Some(branch),
        coordinator_cursor: Some(first),
        coordinator_claim: None,
        coordinator_generation: 1,
        root_nodes: vec![first],
        history: Vec::new(),
    }
}

/// Builds a planned node with a stable structural parent.
///
/// - `id`: Durable node identity.
/// - `parent`: Structural parent relation.
///
/// # Returns
/// A ready plan node for projection assertions.
pub(super) fn node(id: EntityId, parent: Option<PlanNodeParent>) -> PlanNode {
    PlanNode {
        id,
        parent,
        title: format!("Work {id}"),
        purpose: "Keep plan semantics intact".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: scriberig_session::PlanNodeStatus::Ready,
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

/// Wraps durable plans in the canonical session snapshot the Plans view reads.
///
/// - `plans`: Durable plans projected by the Plans view.
///
/// # Returns
/// A session snapshot with no branches and the supplied plans.
pub(super) fn session_with_plans(plans: Vec<Plan>) -> scriberig_session::Session {
    scriberig_session::Session {
        id: EntityId::from_bytes([9; 16]),
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans,
        execution: scriberig_session::SessionExecutionSelection::default(),
    }
}
