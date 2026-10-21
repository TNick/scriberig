//! Verifies stable plan projection and separate dependency-edge presentation.

use super::*;

/// Builds a deterministic plan with structural and dependency relations diverging.
///
/// - `plan_id`: Durable plan identity.
/// - `branch`: Structural parent branch identity.
/// - `first`: First durable node identity.
/// - `second`: Second durable node identity.
///
/// # Returns
/// A plan with one structural edge and a reverse dependency edge.
fn sample_plan(plan_id: EntityId, branch: EntityId, first: EntityId, second: EntityId) -> Plan {
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
fn node(id: EntityId, parent: Option<PlanNodeParent>) -> PlanNode {
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
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}

/// Verifies the tree uses structural parents while Markdown shows DAG edges.
#[test]
fn structural_tree_and_dependency_edges_remain_distinct() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let plan = sample_plan(plan_id, branch, first, second);
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

    let session = scriberig_session::Session {
        id: EntityId::from_bytes([9; 16]),
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let details = plan_details(
        &session,
        Some(PlanSelection {
            plan: plan_id,
            node: Some(second),
        }),
        &rows,
    );
    assert!(details.contains(&format!("{second} → {first}")));
}

/// Verifies node selection resolves by stable plan/node identity after reorder.
#[test]
fn tree_selection_survives_plan_node_reordering() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let mut plan = sample_plan(plan_id, branch, first, second);
    plan.nodes.reverse();
    let state = TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: EntityId::from_bytes([9; 16]),
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        plan_cursor: Some(PlanSelection {
            plan: plan_id,
            node: Some(first),
        }),
        ..TuiState::default()
    };

    let (tree, _, _) = plan_tree(&state).expect("plan tree");

    assert_eq!(tree.selected_id(), Some(plan_row_id(plan_id, Some(first))));
}
