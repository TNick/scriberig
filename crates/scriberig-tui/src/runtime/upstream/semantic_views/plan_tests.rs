//! Verifies stable plan and node identity through the upstream plan tree.

use super::test_support::{sample_plan, session_with_plans};
use super::*;

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
        session_snapshot: Some(session_with_plans(vec![plan])),
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
        session_snapshot: Some(session_with_plans(vec![sample_plan(
            plan_id, branch, first, second,
        )])),
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
        session_snapshot: Some(session_with_plans(vec![plan.clone(), plan])),
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
