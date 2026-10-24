//! Verifies the plan tree keeps structural parentage separate from dependency
//! edges, which are projected only as plan knowledge in the selected details.

use super::test_support::{node, sample_plan, session_with_plans};
use super::*;

/// Verifies the tree uses structural parents while Markdown shows DAG edges.
#[test]
fn structural_tree_and_dependency_edges_remain_distinct() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let mut plan = sample_plan(plan_id, branch, first, second);
    plan.nodes[0].superseded_by = Some(second);
    plan.history.push(scriberig_session::PlanRevision {
        revision: 3,
        goal: "Previous graph goal".to_owned(),
        status: scriberig_session::PlanStatus::Paused,
        nodes: vec![node(first, None)],
        dependencies: Vec::new(),
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        reason: Some("Split the root work into focused nodes".to_owned()),
    });
    let rows = rows_for_plans(std::slice::from_ref(&plan));
    let first_row = rows
        .iter()
        .find(|row| row.selection.node == Some(first))
        .expect("first node row");
    let second_row = rows
        .iter()
        .find(|row| row.selection.node == Some(second))
        .expect("second node row");
    assert_eq!(second_row.parent_id.as_deref(), Some(first_row.id.as_str()));

    let session = session_with_plans(vec![plan]);
    let details = plan_details(
        &session,
        Some(PlanSelection {
            plan: plan_id,
            node: Some(second),
        }),
        &rows,
    );
    assert!(details.contains(&format!("{second} → {first}")));
    assert!(details.contains("Replan reason: Split the root work into focused nodes"));
    let node_details = plan_details(
        &session,
        Some(PlanSelection {
            plan: plan_id,
            node: Some(first),
        }),
        &rows,
    );
    assert!(node_details.contains(&format!("Superseded by node: {second}")));
}

/// Verifies an explicit dependency edge never nests the dependent node beneath
/// its prerequisite, and that the parentage survives a durable node reorder.
#[test]
fn dependency_edges_never_become_structural_tree_parents() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let prerequisite = EntityId::from_bytes([3; 16]);
    let dependent = EntityId::from_bytes([4; 16]);
    let result = EntityId::from_bytes([5; 16]);
    let mut plan = sample_plan(plan_id, branch, prerequisite, dependent);
    plan.nodes = vec![node(prerequisite, None), node(dependent, None)];
    plan.root_nodes = vec![prerequisite];
    plan.dependencies = vec![scriberig_session::PlanDependency {
        prerequisite,
        dependent,
        fulfilled_by: Some(result),
    }];

    let rows = rows_for_plans(std::slice::from_ref(&plan));
    let header = rows
        .iter()
        .find(|row| row.selection.node.is_none())
        .expect("plan header row");
    let prerequisite_row = rows
        .iter()
        .find(|row| row.selection.node == Some(prerequisite))
        .expect("prerequisite node row");
    let dependent_row = rows
        .iter()
        .find(|row| row.selection.node == Some(dependent))
        .expect("dependent node row");

    assert_eq!(
        prerequisite_row.parent_id.as_deref(),
        Some(header.id.as_str())
    );
    assert_eq!(dependent_row.parent_id.as_deref(), Some(header.id.as_str()));
    assert_ne!(
        dependent_row.parent_id.as_deref(),
        Some(prerequisite_row.id.as_str()),
        "a dependency edge must not become structural parentage"
    );
    tree_from_items(
        &rows
            .iter()
            .map(|row| TreeItem {
                id: row.id.clone(),
                label: row.id.clone(),
                parent_id: row.parent_id.clone(),
            })
            .collect::<Vec<_>>(),
    )
    .expect("upstream tree accepts the knowledge-derived parentage");

    // The durable node vector is display order only, so reversing it must keep
    // the same parentage while the explicit edge stays in the selected details.
    let mut reordered = plan.clone();
    reordered.nodes.reverse();
    let reordered_rows = rows_for_plans(&[reordered]);
    let reordered_dependent = reordered_rows
        .iter()
        .find(|row| row.selection.node == Some(dependent))
        .expect("dependent node row after reordering");
    let reordered_prerequisite = reordered_rows
        .iter()
        .find(|row| row.selection.node == Some(prerequisite))
        .expect("prerequisite node row after reordering");
    assert_eq!(
        reordered_dependent.parent_id, dependent_row.parent_id,
        "projected parentage must not follow the durable node vector order"
    );
    assert_ne!(
        reordered_dependent.parent_id.as_deref(),
        Some(reordered_prerequisite.id.as_str())
    );

    let details = plan_details(
        &session_with_plans(vec![plan]),
        Some(PlanSelection {
            plan: plan_id,
            node: Some(dependent),
        }),
        &rows,
    );
    assert!(details.contains(&format!("{prerequisite} → {dependent}")));
    assert!(details.contains(&format!("result {result}")));
}

/// Verifies structural parentage comes from `PlanNodeParent` even when the
/// durable node vector lists a child before its parent.
#[test]
fn structural_parent_is_resolved_from_plan_knowledge_not_row_order() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let parent_node = EntityId::from_bytes([3; 16]);
    let child_node = EntityId::from_bytes([4; 16]);
    let mut plan = sample_plan(plan_id, branch, parent_node, child_node);
    plan.nodes = vec![
        node(child_node, Some(PlanNodeParent::Node(parent_node))),
        node(parent_node, None),
    ];
    plan.root_nodes = vec![parent_node];
    // A reverse dependency edge points opposite to the structure, so a
    // projection that confused the two relations would nest the parent instead.
    plan.dependencies = vec![scriberig_session::PlanDependency {
        prerequisite: child_node,
        dependent: parent_node,
        fulfilled_by: None,
    }];

    let rows = rows_for_plans(std::slice::from_ref(&plan));
    let header = rows
        .iter()
        .find(|row| row.selection.node.is_none())
        .expect("plan header row");
    let parent_row = rows
        .iter()
        .find(|row| row.selection.node == Some(parent_node))
        .expect("structural parent row");
    let child_row = rows
        .iter()
        .find(|row| row.selection.node == Some(child_node))
        .expect("structural child row");

    assert_eq!(parent_row.parent_id.as_deref(), Some(header.id.as_str()));
    assert_eq!(child_row.parent_id.as_deref(), Some(parent_row.id.as_str()));
    let parent_position = rows
        .iter()
        .position(|row| row.id == parent_row.id)
        .expect("projected parent position");
    let child_position = rows
        .iter()
        .position(|row| row.id == child_row.id)
        .expect("projected child position");
    assert!(
        parent_position < child_position,
        "the structural parent must be projected before its child: {:?}",
        rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>()
    );

    let parent_details = plan_details(
        &session_with_plans(vec![plan.clone()]),
        Some(PlanSelection {
            plan: plan_id,
            node: Some(parent_node),
        }),
        &rows,
    );
    // The reverse edge is knowledge only: it accompanies the parent without
    // becoming its tree parent, which the row assertions above established.
    assert!(parent_details.contains(&format!("{child_node} → {parent_node}")));
    assert!(parent_details.contains("Structural parent: none"));
    let child_details = plan_details(
        &session_with_plans(vec![plan]),
        Some(PlanSelection {
            plan: plan_id,
            node: Some(child_node),
        }),
        &rows,
    );
    assert!(child_details.contains(&format!("Structural parent node: {parent_node}")));
}
