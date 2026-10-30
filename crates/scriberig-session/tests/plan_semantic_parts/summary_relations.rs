//! Verifies node summaries remain separate canonical relations.

use scriberig_core::EntityId;
use scriberig_session::{
    ContinuationAnchor, Plan, PlanNode, PlanNodeParent, PlanNodeStatus, PlanNodeSummary,
    PlanSemanticReference,
};

/// Keeps child summary text out of the parent's canonical summary.
#[test]
fn child_summary_remains_distinct_from_parent_summary() {
    let branch = EntityId::from_bytes([1; 16]);
    let parent = EntityId::from_bytes([2; 16]);
    let child = EntityId::from_bytes([3; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([4; 16]),
        EntityId::from_bytes([5; 16]),
        ContinuationAnchor::BranchHead { branch },
        "separate summaries",
    )
    .expect("plan");
    plan.add_node(node(parent, PlanNodeParent::Branch(branch), "parent"))
        .expect("parent node");
    plan.add_node(node(child, PlanNodeParent::Node(parent), "child"))
        .expect("child node");
    plan.update_summary(PlanNodeSummary {
        id: EntityId::from_bytes([6; 16]),
        node: parent,
        text: "Parent's independent conclusion".to_owned(),
        sources: vec![reference(7)],
    })
    .expect("parent summary");
    plan.update_summary(PlanNodeSummary {
        id: EntityId::from_bytes([8; 16]),
        node: child,
        text: "Child detail that remains separately addressable".to_owned(),
        sources: vec![reference(9)],
    })
    .expect("child summary");

    assert_eq!(plan.summaries.len(), 2);
    assert_eq!(plan.summaries[0].node, parent);
    assert_eq!(plan.summaries[0].text, "Parent's independent conclusion");
    assert_eq!(plan.summaries[1].node, child);
    assert_eq!(
        plan.summaries[1].text,
        "Child detail that remains separately addressable"
    );
    assert!(!plan.summaries[0].text.contains(&plan.summaries[1].text));
}

/// Creates a planned node at one structural parent.
///
/// # Parameters
/// - `id`: Stable plan node identity.
/// - `parent`: Structural parent relation.
/// - `title`: Work title and purpose.
///
/// # Returns
/// A planned node without execution state.
fn node(id: EntityId, parent: PlanNodeParent, title: &str) -> PlanNode {
    PlanNode {
        id,
        parent: Some(parent),
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

/// Creates an exact source reference for a summary.
///
/// # Parameters
/// - `id`: Entity identity byte repeated across the ID.
///
/// # Returns
/// A semantic source reference without a pinned revision or snapshot.
fn reference(id: u8) -> PlanSemanticReference {
    PlanSemanticReference {
        entity: EntityId::from_bytes([id; 16]),
        revision: None,
        snapshot: None,
    }
}
