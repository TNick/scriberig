//! Verifies semantic identity and selection retention across authoritative
//! conversation and session refreshes: historical continuation relocation, row
//! reorder, and reuse of one retained upstream tree.

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
