//! Verifies stable plan and node identity through the upstream plan tree.

use super::super::projection::sync_tree;
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

/// Verifies the Plans view reveals node rows whose plan header the conversation
/// view projected without children.
#[test]
fn plan_view_reveals_nodes_under_a_header_the_conversation_view_left_childless() {
    let plan_id = EntityId::from_bytes([11; 16]);
    let branch = EntityId::from_bytes([12; 16]);
    let first = EntityId::from_bytes([13; 16]);
    let second = EntityId::from_bytes([14; 16]);
    let mut plan = sample_plan(plan_id, branch, first, second);
    // Anchoring at a branch makes the conversation projection group plan nodes by
    // their structural branch or node parent, so a node with no explicit structural
    // parent is not projected there and the shared plan header row stays childless.
    plan.anchor = Some(scriberig_session::ContinuationAnchor::BranchHead { branch });
    let mut session = session_with_plans(vec![plan]);
    let mut host =
        scriberig_session::Branch::new(branch, "Plan host branch").expect("create host branch");
    host.status = scriberig_session::BranchStatus::Completed;
    session.branches.push(host);

    let header_key = plan_row_id(plan_id, None);
    let conversation_state = TuiState {
        primary_view: crate::selection::PrimaryView::Conversation,
        conversation: Some(crate::view::conversation::ConversationModel::from_session(
            &session,
        )),
        session_snapshot: Some(session.clone()),
        ..TuiState::default()
    };
    let before = super::super::tree::projection(&conversation_state)
        .expect("conversation tree projection with a plan row");
    assert!(
        before.items.iter().any(|item| item.id == header_key),
        "both views key the durable plan by the same stable row identity: {:?}",
        before
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        before
            .items
            .iter()
            .all(|item| item.parent_id.as_deref() != Some(header_key.as_str())),
        "the conversation view leaves that plan header childless"
    );

    let plan_state = TuiState {
        primary_view: crate::selection::PrimaryView::Plan,
        session_snapshot: Some(session),
        ..TuiState::default()
    };
    let (after, rows) = projection(&plan_state).expect("plan tree projection");
    assert_eq!(rows.len(), 3, "one header and two structural node rows");

    // One retained tree serves both views, so the inherited collapsed header must
    // still reveal the node rows the Plans view projects beneath it.
    let mut slot = None;
    sync_tree(&mut slot, &before).expect("tree accepts the conversation projection");
    sync_tree(&mut slot, &after).expect("the retained tree accepts the plan projection");
    let visible = slot
        .as_ref()
        .expect("retained tree")
        .tree
        .visible_rows()
        .iter()
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    for expected in [
        header_key,
        plan_row_id(plan_id, Some(first)),
        plan_row_id(plan_id, Some(second)),
    ] {
        assert!(
            visible.iter().any(|id| id == &expected),
            "the Plans view must render {expected}: {visible:?}"
        );
    }
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
