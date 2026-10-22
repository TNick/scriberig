//! Verifies status precedence for reference plan rollups.

use super::*;
use scriberig_session::{PlanAbandonReason, PlanGate};

/// Describes one direct and required-child rollup case.
struct Case {
    /// Direct status of the parent node.
    parent: PlanNodeStatus,
    /// Direct status of the required child node.
    child: PlanNodeStatus,
    /// Whether to mark the parent abandoned explicitly.
    abandon_parent: bool,
    /// Whether to mark the child ignored.
    abandon_child: bool,
    /// Expected effective parent status.
    expected: PlanNodeStatus,
    /// Whether the parent rollup contains ignored work.
    contains_ignored: bool,
}

/// Builds one parent and required child, then checks its reference rollup.
///
/// # Parameters
/// - `identity`: Stable byte used to derive case entities.
/// - `case`: Direct statuses and expected aggregate values.
fn assert_reference_case(identity: u8, case: &Case) {
    let branch = EntityId::from_bytes([240; 16]);
    let parent_id = EntityId::from_bytes([identity + 10; 16]);
    let child_id = EntityId::from_bytes([identity + 30; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([identity + 50; 16]),
        EntityId::from_bytes([identity + 70; 16]),
        ContinuationAnchor::BranchHead { branch },
        "rollup precedence",
    )
    .expect("plan");
    let parent = node(identity + 10, branch, "parent");
    let mut child = node(identity + 30, branch, "required child");
    child.parent = Some(PlanNodeParent::Node(parent_id));
    plan.add_node(parent).expect("parent");
    plan.add_node(child).expect("child");
    plan.add_gate(PlanGate {
        id: EntityId::from_bytes([identity + 90; 16]),
        owner: parent_id,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::RequiredChildCompleted { child: child_id },
        waiver: None,
    })
    .expect("completion gate");
    let parent = plan
        .nodes
        .iter_mut()
        .find(|node| node.id == parent_id)
        .expect("parent node");
    parent.status = case.parent;
    if case.abandon_parent {
        parent.abandon_reason = Some(PlanAbandonReason::Cancelled);
    }
    let child = plan
        .nodes
        .iter_mut()
        .find(|node| node.id == child_id)
        .expect("child node");
    child.status = case.child;
    if case.abandon_child {
        child.abandon_reason = Some(PlanAbandonReason::Ignored);
    }
    let rollup = plan.node_rollups().expect("reference rollups")[&parent_id];
    assert_eq!(rollup.effective_status, case.expected);
    assert_eq!(rollup.contains_ignored, case.contains_ignored);
}

/// Matches table-driven direct and required-child states to effective status.
#[test]
fn reference_rollups_follow_required_status_precedence() {
    let cases = [
        Case {
            parent: PlanNodeStatus::Planned,
            child: PlanNodeStatus::Completed,
            abandon_parent: false,
            abandon_child: false,
            expected: PlanNodeStatus::Planned,
            contains_ignored: false,
        },
        Case {
            parent: PlanNodeStatus::Completed,
            child: PlanNodeStatus::Ready,
            abandon_parent: false,
            abandon_child: false,
            expected: PlanNodeStatus::Ready,
            contains_ignored: false,
        },
        Case {
            parent: PlanNodeStatus::Completed,
            child: PlanNodeStatus::Running,
            abandon_parent: false,
            abandon_child: false,
            expected: PlanNodeStatus::Running,
            contains_ignored: false,
        },
        Case {
            parent: PlanNodeStatus::Failed,
            child: PlanNodeStatus::Running,
            abandon_parent: false,
            abandon_child: false,
            expected: PlanNodeStatus::Failed,
            contains_ignored: false,
        },
        Case {
            parent: PlanNodeStatus::Completed,
            child: PlanNodeStatus::Failed,
            abandon_parent: false,
            abandon_child: false,
            expected: PlanNodeStatus::Failed,
            contains_ignored: false,
        },
        Case {
            parent: PlanNodeStatus::Abandoned,
            child: PlanNodeStatus::Running,
            abandon_parent: true,
            abandon_child: false,
            expected: PlanNodeStatus::Abandoned,
            contains_ignored: false,
        },
        Case {
            parent: PlanNodeStatus::Completed,
            child: PlanNodeStatus::Abandoned,
            abandon_parent: false,
            abandon_child: true,
            expected: PlanNodeStatus::Planned,
            contains_ignored: true,
        },
        Case {
            parent: PlanNodeStatus::Completed,
            child: PlanNodeStatus::Completed,
            abandon_parent: false,
            abandon_child: false,
            expected: PlanNodeStatus::Completed,
            contains_ignored: false,
        },
    ];

    for (index, case) in cases.into_iter().enumerate() {
        assert_reference_case(u8::try_from(index).expect("case identity"), &case);
    }
}
