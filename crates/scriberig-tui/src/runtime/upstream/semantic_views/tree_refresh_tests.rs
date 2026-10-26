//! Verifies semantic identity and selection retention across authoritative
//! conversation and session refreshes: historical continuation relocation, row
//! reorder, reuse of one retained upstream tree, reducer-requested expansion,
//! and the collapse state that reuse must preserve.

use super::super::projection::sync_tree;
use super::test_support::{
    authoritative_conversation_refresh, conversation_session, conversation_state,
    replace_session_rows, session_row, session_state,
};
use super::*;
use crate::view::conversation::{ConversationModel, ConversationSource};
use crate::view::session::{SessionRowIdentity, SessionRowKind};
use scriberig_core::EntityId;

/// Verifies a retained conversation source stays selected and a historical
/// continuation stays anchored to its own row after an authoritative rerender.
#[test]
fn retained_conversation_identity_survives_an_authoritative_rerender() {
    let session = EntityId::from_bytes([71; 16]);
    let root = EntityId::from_bytes([72; 16]);
    let anchor_turn = EntityId::from_bytes([73; 16]);
    let later_turn = EntityId::from_bytes([74; 16]);
    let continuation = EntityId::from_bytes([75; 16]);
    let sibling = EntityId::from_bytes([76; 16]);
    let anchor_key = format!("activity:{anchor_turn}");
    let value = conversation_session(session, root, &[anchor_turn, later_turn]);

    let before = ConversationModel::from_session(&value);
    let mut state = conversation_state(before.clone(), anchor_turn);
    let before_projection = projection(&state).expect("conversation tree projection");
    assert_eq!(
        before_projection.selected_id.as_deref(),
        Some(anchor_key.as_str())
    );

    // The refresh relocates the continuation beneath its historical anchor and
    // shifts every later flattened position.
    let mut refreshed = value;
    authoritative_conversation_refresh(&mut refreshed, root, anchor_turn, continuation, sibling);
    state.conversation = Some(ConversationModel::from_session(&refreshed));
    // A stale flattened position must not replace the retained semantic identity.
    state.cursor = before
        .index_of(ConversationSource::Activity(later_turn))
        .expect("later activity before the refresh");

    let after = projection(&state).expect("rerendered conversation tree projection");
    let ids = after
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(after.selected_id.as_deref(), Some(anchor_key.as_str()));
    let anchor_position = ids
        .iter()
        .position(|id| *id == anchor_key)
        .expect("anchor activity row");
    let continuation_position = ids
        .iter()
        .position(|id| *id == format!("branch:{continuation}"))
        .expect("continuation branch row keeps its own identity");
    let later_position = ids
        .iter()
        .position(|id| *id == format!("activity:{later_turn}"))
        .expect("later activity row");
    assert!(
        anchor_position < continuation_position && continuation_position < later_position,
        "the continuation must stay anchored between its anchor row and the next row: {ids:?}"
    );

    let mut slot = None;
    sync_tree(&mut slot, &before_projection).expect("tree accepts the first projection");
    sync_tree(&mut slot, &after).expect("the retained tree accepts the rerender");
    assert_eq!(
        slot.as_ref()
            .expect("retained tree")
            .tree
            .selected_id()
            .as_deref(),
        Some(anchor_key.as_str()),
        "the reused upstream tree must follow the semantic identity, not the old position"
    );
}

/// Verifies a reused upstream tree keeps its semantic selection and collapse
/// state when an authoritative refresh reorders the projected rows.
#[test]
fn retained_tree_keeps_identity_and_collapse_through_an_authoritative_reorder() {
    let session = EntityId::from_bytes([81; 16]);
    let parent = EntityId::from_bytes([82; 16]);
    let child = EntityId::from_bytes([83; 16]);
    let sibling = EntityId::from_bytes([84; 16]);
    let parent_key = format!("session-row:Branch:{parent}");
    let child_key = format!("session-row:Branch:{child}");
    let sibling_key = format!("session-row:Branch:{sibling}");
    let mut state = session_state(vec![
        session_row(SessionRowKind::Session, session, 0),
        session_row(SessionRowKind::Branch, parent, 1),
        session_row(SessionRowKind::Branch, child, 2),
        session_row(SessionRowKind::Branch, sibling, 1),
    ]);
    state.session_cursor = Some(SessionRowIdentity {
        reference: parent,
        kind: SessionRowKind::Branch,
    });

    let before = projection(&state).expect("session tree projection");
    let mut slot = None;
    sync_tree(&mut slot, &before).expect("tree accepts the first projection");
    slot.as_ref()
        .expect("retained tree")
        .tree
        .set_expanded(&parent_key, false);

    // The refresh moves the sibling ahead of the selected branch, so the old
    // flattened position now addresses a different semantic row.
    replace_session_rows(
        &mut state,
        vec![
            session_row(SessionRowKind::Session, session, 0),
            session_row(SessionRowKind::Branch, sibling, 1),
            session_row(SessionRowKind::Branch, parent, 1),
            session_row(SessionRowKind::Branch, child, 2),
        ],
    );
    let after = projection(&state).expect("reordered session tree projection");
    sync_tree(&mut slot, &after).expect("the retained tree accepts the reorder");

    let retained = slot.as_ref().expect("retained tree");
    let visible = retained
        .tree
        .visible_rows()
        .iter()
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        retained.tree.selected_id().as_deref(),
        Some(parent_key.as_str())
    );
    assert_eq!(
        visible.iter().position(|id| id == &sibling_key),
        Some(1),
        "the reordered row must take its new visible position: {visible:?}"
    );
    assert!(
        !visible.iter().any(|id| id == &child_key),
        "collapse state belongs to the stable identity, not a row position: {visible:?}"
    );
}

/// Builds one completed leaf branch with one answered turn.
///
/// - `id`: Stable leaf branch identity.
/// - `parent`: Structural parent branch identity.
/// - `turn`: Durable turn identity owned by the leaf branch.
///
/// A completed child branch that projects no structural child row, which is the
/// shape whose activity rows stay hidden until the semantic reducer expands it.
/// # Returns
/// The durable leaf branch value.
fn completed_leaf_branch(
    id: EntityId,
    parent: EntityId,
    turn: EntityId,
) -> scriberig_session::Branch {
    let mut branch =
        scriberig_session::Branch::new(id, "Deepest leaf branch").expect("create leaf branch");
    branch.parent = Some(parent);
    branch.status = scriberig_session::BranchStatus::Completed;
    branch.push_turn(scriberig_session::Turn {
        id: turn,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "deepest leaf question".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "leaf answer".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    branch
}

/// Verifies expanding a completed leaf branch reveals the activity rows its
/// semantic projection materialized under an already projected row.
#[test]
fn expanding_a_completed_leaf_branch_reveals_its_materialized_activity_rows() {
    let session = EntityId::from_bytes([91; 16]);
    let root = EntityId::from_bytes([92; 16]);
    let root_turn = EntityId::from_bytes([93; 16]);
    let leaf = EntityId::from_bytes([94; 16]);
    let leaf_turn = EntityId::from_bytes([95; 16]);
    let mut value = conversation_session(session, root, &[root_turn]);
    value
        .branches
        .push(completed_leaf_branch(leaf, root, leaf_turn));

    let mut state = conversation_state(ConversationModel::from_session(&value), root_turn);
    state.conversation_cursor = Some(ConversationSource::Branch(leaf));
    let before = projection(&state).expect("compact conversation projection");
    let leaf_key = format!("branch:{leaf}");
    let activity_key = format!("activity:{leaf_turn}");
    assert!(
        before.items.iter().any(|item| item.id == leaf_key),
        "the completed leaf branch keeps its own row: {:?}",
        item_shapes(&before)
    );
    assert!(
        before
            .items
            .iter()
            .all(|item| item.parent_id.as_deref() != Some(leaf_key.as_str())),
        "a completed leaf branch projects no children while compact: {:?}",
        item_shapes(&before)
    );

    // `conversation_toggle_expansion` rebuilds this projection with the branch in
    // `expanded_branches`; the branch row itself keeps its stable identity.
    state.conversation = Some(ConversationModel::from_session_with_expanded(
        &value,
        &std::collections::BTreeSet::from([leaf]),
    ));
    let after = projection(&state).expect("expanded conversation projection");
    assert!(
        after.items.iter().any(|item| item.id == activity_key),
        "the semantic projection must materialize the activity row: {:?}",
        item_shapes(&after)
    );

    let mut slot = None;
    sync_tree(&mut slot, &before).expect("tree accepts the compact projection");
    sync_tree(&mut slot, &after).expect("the retained tree accepts the expansion");
    let retained = slot.as_ref().expect("retained tree");
    let visible = visible_ids(&retained.tree);
    assert!(
        visible.iter().any(|id| id == &leaf_key),
        "the stable branch row stays visible: {visible:?}"
    );
    assert!(
        visible.iter().any(|id| id == &activity_key),
        "expanding a completed leaf branch must reveal its activity rows: {visible:?}"
    );
    assert_eq!(
        retained.tree.selected_id().as_deref(),
        Some(leaf_key.as_str()),
        "expansion keeps the stable branch row selected"
    );
}

/// Verifies a collapse bound to a stable identity still survives a projection
/// that adds another child row beneath that already-parented row.
#[test]
fn a_retained_collapse_survives_a_new_child_under_the_collapsed_row() {
    let session = EntityId::from_bytes([96; 16]);
    let parent = EntityId::from_bytes([97; 16]);
    let child = EntityId::from_bytes([98; 16]);
    let added = EntityId::from_bytes([99; 16]);
    let parent_key = format!("session-row:Branch:{parent}");
    let child_key = format!("session-row:Branch:{child}");
    let added_key = format!("session-row:Branch:{added}");
    let mut state = session_state(vec![
        session_row(SessionRowKind::Session, session, 0),
        session_row(SessionRowKind::Branch, parent, 1),
        session_row(SessionRowKind::Branch, child, 2),
    ]);

    let before = projection(&state).expect("session projection with one child row");
    let mut slot = None;
    sync_tree(&mut slot, &before).expect("tree accepts the first projection");
    slot.as_ref()
        .expect("retained tree")
        .tree
        .set_expanded(&parent_key, false);

    replace_session_rows(
        &mut state,
        vec![
            session_row(SessionRowKind::Session, session, 0),
            session_row(SessionRowKind::Branch, parent, 1),
            session_row(SessionRowKind::Branch, child, 2),
            session_row(SessionRowKind::Branch, added, 2),
        ],
    );
    let after = projection(&state).expect("session projection with a second child row");
    assert!(
        after.items.iter().any(|item| item.id == added_key),
        "the refresh adds a child row: {:?}",
        item_shapes(&after)
    );
    sync_tree(&mut slot, &after).expect("the retained tree accepts the new child row");

    let visible = visible_ids(&slot.as_ref().expect("retained tree").tree);
    assert!(
        !visible.iter().any(|id| id == &child_key) && !visible.iter().any(|id| id == &added_key),
        "collapse belongs to the already-parented row's stable identity: {visible:?}"
    );
}

/// Returns the projected upstream item ID and parent pairs of one projection.
///
/// - `projection`: Semantic tree projection to describe in an assertion.
///
/// The pairs used as assertion context.
/// # Returns
/// One `(id, parent_id)` pair per projected item.
fn item_shapes(projection: &TreeProjection) -> Vec<(&str, Option<&str>)> {
    projection
        .items
        .iter()
        .map(|item| (item.id.as_str(), item.parent_id.as_deref()))
        .collect()
}

/// Returns the visible upstream row IDs of one retained tree.
///
/// - `tree`: Retained upstream tree to inspect.
///
/// The visible row IDs in display order.
/// # Returns
/// One ID per visible row.
fn visible_ids(tree: &rpi_tui::TreeList) -> Vec<String> {
    tree.visible_rows()
        .iter()
        .map(|row| row.id.clone())
        .collect()
}
