//! Verifies plan-worker roles follow canonical session branch topology.

use super::PlanWorkerContext;
use crate::context::BranchRole;
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, ContinuationAnchor, Plan, PlanNode, PlanNodeParent, PlanNodeStatus, Session,
    WorkerClaim,
};

/// Uses persisted branch parents even when plan-node nesting differs.
#[test]
fn plan_worker_role_comes_from_claimed_branch_parent() {
    let session_id = EntityId::from_bytes([1; 16]);
    let root_branch = EntityId::from_bytes([2; 16]);
    let plan_parent_branch = EntityId::from_bytes([3; 16]);
    let actual_parent_branch = EntityId::from_bytes([4; 16]);
    let root_worker_branch = EntityId::from_bytes([5; 16]);
    let child_worker_branch = EntityId::from_bytes([6; 16]);
    let plan_id = EntityId::from_bytes([7; 16]);
    let plan_parent_node = EntityId::from_bytes([8; 16]);
    let root_worker_node = EntityId::from_bytes([9; 16]);
    let child_worker_node = EntityId::from_bytes([10; 16]);

    let session = Session {
        id: session_id,
        branches: vec![
            Branch::new(root_branch, "root").expect("root branch"),
            Branch::new_child(
                plan_parent_branch,
                root_branch,
                "plan parent",
                "plan parent work",
            )
            .expect("plan parent branch"),
            Branch::new_child(
                actual_parent_branch,
                root_branch,
                "actual parent",
                "actual parent work",
            )
            .expect("actual parent branch"),
            Branch::new(root_worker_branch, "root worker").expect("root worker branch"),
            Branch::new_child(
                child_worker_branch,
                actual_parent_branch,
                "child worker",
                "child worker work",
            )
            .expect("child worker branch"),
        ],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "execute workers",
    )
    .expect("plan");
    plan.nodes = vec![
        plan_node(
            plan_parent_node,
            Some(PlanNodeParent::Branch(root_branch)),
            Some(plan_parent_branch),
        ),
        plan_node(
            root_worker_node,
            Some(PlanNodeParent::Node(plan_parent_node)),
            Some(root_worker_branch),
        ),
        plan_node(
            child_worker_node,
            Some(PlanNodeParent::Node(root_worker_node)),
            Some(child_worker_branch),
        ),
    ];
    let root_claim = worker_claim(root_worker_node, root_worker_branch);
    let child_claim = worker_claim(child_worker_node, child_worker_branch);

    let root_context = PlanWorkerContext::from_plan(&session, &plan, &plan.nodes[1], &root_claim)
        .expect("root worker context");
    let child_context = PlanWorkerContext::from_plan(&session, &plan, &plan.nodes[2], &child_claim)
        .expect("child worker context");

    assert_eq!(root_context.inputs.branch_role, BranchRole::Root);
    assert_eq!(
        child_context.inputs.branch_role,
        BranchRole::NonRoot {
            parent: actual_parent_branch.to_string(),
        }
    );
    assert_ne!(
        child_context.inputs.branch_role,
        BranchRole::NonRoot {
            parent: root_worker_branch.to_string(),
        }
    );
    assert!(
        child_context
            .compiled
            .canonical
            .contains("scriberig.context-component.non-root-handoff@")
    );
}

/// Assigns fallback workers the same parent as coordinator branch creation.
#[test]
fn minimal_plan_worker_context_uses_coordinator_parent_rule() {
    let session_id = EntityId::from_bytes([21; 16]);
    let root_branch = EntityId::from_bytes([22; 16]);
    let assigned_parent_branch = EntityId::from_bytes([23; 16]);
    let worker_branch = EntityId::from_bytes([24; 16]);
    let plan_id = EntityId::from_bytes([25; 16]);
    let parent_node_id = EntityId::from_bytes([26; 16]);
    let worker_node_id = EntityId::from_bytes([27; 16]);
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "execute workers",
    )
    .expect("plan");
    plan.nodes = vec![
        plan_node(
            parent_node_id,
            Some(PlanNodeParent::Branch(root_branch)),
            Some(assigned_parent_branch),
        ),
        plan_node(
            worker_node_id,
            Some(PlanNodeParent::Node(parent_node_id)),
            Some(worker_branch),
        ),
    ];
    let claim = worker_claim(worker_node_id, worker_branch);

    let context = PlanWorkerContext::minimal(&plan, &plan.nodes[1], &claim);

    assert_eq!(
        context.inputs.branch_role,
        BranchRole::NonRoot {
            parent: assigned_parent_branch.to_string(),
        }
    );
    assert!(
        context
            .compiled
            .canonical
            .contains("scriberig.context-component.non-root-handoff@")
    );
}

/// Creates a minimal plan node for branch-role context coverage.
///
/// - `id`: Stable plan-node identity.
/// - `parent`: Structural plan parent, independent of branch parentage here.
/// - `branch`: Branch assigned to the plan node.
///
/// A ready node bound to the supplied branch.
/// # Returns
/// The plan node fixture.
fn plan_node(id: EntityId, parent: Option<PlanNodeParent>, branch: Option<EntityId>) -> PlanNode {
    PlanNode {
        id,
        parent,
        title: "worker".to_owned(),
        purpose: "perform focused work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Ready,
        branch,
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}

/// Creates a worker claim for one exact node and branch.
///
/// - `node`: Claimed plan node.
/// - `branch`: Persisted worker branch.
///
/// A claim matching the supplied node and branch.
/// # Returns
/// The worker claim fixture.
fn worker_claim(node: EntityId, branch: EntityId) -> WorkerClaim {
    WorkerClaim {
        id: EntityId::from_bytes([11; 16]),
        node,
        branch,
        owner: "fixture".to_owned(),
        generation: 1,
        assignment: None,
    }
}
