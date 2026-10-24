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
        abandon_reason: None,
        last_failure: None,
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

/// Keeps highlighted and detailed plan selections aligned after cursor fallback.
#[test]
fn tree_and_details_share_fallback_plan_selection() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let state = TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: EntityId::from_bytes([9; 16]),
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![sample_plan(plan_id, branch, first, second)],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        cursor: 2,
        ..TuiState::default()
    };

    let (tree, details, _) = plan_tree(&state).expect("plan tree");

    assert_eq!(tree.selected_id(), Some(plan_row_id(plan_id, Some(second))));
    assert!(details.contains(&format!("## Node {second}")));
}

/// Verifies repeated durable plan and node identities still project distinct
/// upstream rows, so one repeated identity cannot blank the whole plan tree.
#[test]
fn duplicate_plan_identities_keep_distinct_plan_tree_rows() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let plan = sample_plan(plan_id, branch, first, second);
    let state = TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: EntityId::from_bytes([9; 16]),
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan.clone(), plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        plan_cursor: Some(PlanSelection {
            plan: plan_id,
            node: Some(first),
        }),
        ..TuiState::default()
    };

    let (tree, details, rows) = plan_tree(&state).expect("plan tree with repeated identities");
    let ids = rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>();
    let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();

    assert_eq!(unique.len(), ids.len(), "{ids:?}");
    assert_eq!(tree.items().len(), ids.len());
    assert_eq!(
        tree.selected_id(),
        Some(plan_row_id(plan_id, Some(first))),
        "the first projected row keeps the exact selection key"
    );
    assert!(details.contains(&format!("## Node {first}")));
}

/// Verifies each projected copy of a repeated plan keeps its own structural
/// parent rows instead of re-parenting nodes under the first copy.
#[test]
fn repeated_plan_identity_keeps_each_copy_under_its_own_plan_header() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let plan = sample_plan(plan_id, branch, first, second);
    let rows = rows_for_plans(&[plan.clone(), plan]);
    let headers = rows
        .iter()
        .filter(|row| row.selection.node.is_none())
        .collect::<Vec<_>>();
    assert_eq!(headers.len(), 2);

    let parent_rows = rows
        .iter()
        .filter(|row| row.selection.node == Some(first))
        .collect::<Vec<_>>();
    let node_rows = rows
        .iter()
        .filter(|row| row.selection.node == Some(second))
        .collect::<Vec<_>>();
    assert_eq!(node_rows.len(), 2);
    let parents = node_rows
        .iter()
        .map(|row| row.parent_id.clone().expect("structural parent"))
        .collect::<Vec<_>>();
    assert_ne!(parents[0], parents[1], "{parents:?}");
    for (index, parent) in parents.iter().enumerate() {
        assert!(
            parent.starts_with(&headers[index].id),
            "copy {index} parent {parent} is not under {} (structural identity was reused)",
            headers[index].id
        );
        assert_eq!(
            parent.as_str(),
            parent_rows[index].id.as_str(),
            "copy {index} must nest under its own projected structural parent row"
        );
    }
}
