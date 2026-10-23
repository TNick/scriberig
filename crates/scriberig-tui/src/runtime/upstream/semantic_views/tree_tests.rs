//! Verifies stable semantic identity and selection retention in the conversation
//! and session tree projections.

use super::super::projection::{sync_tree, tree_from_items};
use super::test_support::{ids_are_unique, replace_session_rows, session_row, session_state};
use super::*;
use crate::view::conversation::ConversationModel;
use crate::view::session::{SessionModel, SessionRowIdentity, SessionRowKind};
use scriberig_core::EntityId;

/// Verifies one durable turn projected as both human input and assistant output
/// keeps two visible rows instead of rejecting the whole conversation tree.
#[test]
fn durable_turn_with_human_and_agent_output_keeps_unique_tree_rows() {
    let session = EntityId::from_bytes([11; 16]);
    let branch = EntityId::from_bytes([12; 16]);
    let turn = EntityId::from_bytes([13; 16]);
    let mut root = scriberig_session::Branch::new(branch, "Durable conversation")
        .expect("create seeded branch");
    root.status = scriberig_session::BranchStatus::Completed;
    root.push_turn(scriberig_session::Turn {
        id: turn,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "first question".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "first answer".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    root.activity
        .push(scriberig_session::ExecutionActivity::HumanInput(turn));
    root.activity
        .push(scriberig_session::ExecutionActivity::AgentOutput(turn));
    let session = scriberig_session::Session {
        id: session,
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let state = TuiState {
        primary_view: crate::selection::PrimaryView::Conversation,
        conversation: Some(ConversationModel::from_session(&session)),
        conversation_cursor: Some(crate::view::conversation::ConversationSource::Activity(
            turn,
        )),
        ..TuiState::default()
    };
    let projection = projection(&state).expect("conversation tree projection");
    let tree =
        tree_from_items(&projection.items).expect("upstream tree must accept the projected rows");
    let semantic_key = format!("activity:{turn}");
    let ids = projection
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();

    assert!(ids_are_unique(&projection), "{ids:?}");
    assert_eq!(
        tree.visible_rows().len(),
        projection.items.len(),
        "both projected turn rows must stay visible"
    );
    assert_eq!(
        projection.selected_id.as_deref(),
        Some(semantic_key.as_str()),
        "the retained conversation source must resolve to its first projected row"
    );
    let duplicate = ids
        .iter()
        .find(|id| **id != semantic_key && id.starts_with(&semantic_key))
        .expect("the duplicate projected row keeps the semantic key as its prefix");
    assert!(duplicate.contains('#'), "{duplicate}");
}

/// Verifies a durable branch result that also lists itself in `references`
/// produces distinct session rows instead of an unrenderable tree.
#[test]
fn self_referencing_branch_result_keeps_unique_session_tree_rows() {
    let session = EntityId::from_bytes([21; 16]);
    let branch = EntityId::from_bytes([22; 16]);
    let result = EntityId::from_bytes([23; 16]);
    let mut root =
        scriberig_session::Branch::new(branch, "Completed agent work").expect("create branch");
    root.status = scriberig_session::BranchStatus::Completed;
    root.result = Some(scriberig_session::BranchResult {
        id: result,
        branch,
        status: scriberig_session::BranchStatus::Completed,
        summary: Some("done".to_owned()),
        result_kind: Some("agent".to_owned()),
        candidate_context: None,
        references: vec![result],
        result_handles: vec!["candidate:c1".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    let model = SessionModel::from_session(&scriberig_session::Session {
        id: session,
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    });
    let expected_rows = model.rows.len();
    let mut state = session_state(model.rows);
    state.session_cursor = Some(SessionRowIdentity {
        reference: result,
        kind: SessionRowKind::Result,
    });

    let projection = projection(&state).expect("session tree projection");
    let tree =
        tree_from_items(&projection.items).expect("upstream tree must accept the projected rows");
    let semantic_key = format!("session-row:Result:{result}");

    assert!(ids_are_unique(&projection));
    assert_eq!(tree.visible_rows().len(), expected_rows);
    assert_eq!(
        projection.selected_id.as_deref(),
        Some(semantic_key.as_str())
    );
    assert!(
        projection
            .items
            .iter()
            .any(|item| item.id.starts_with("session-row:Candidate:")),
        "the review candidate row must stay projected for activation"
    );
}

/// Verifies an authoritative reorder keeps the retained session row identity
/// highlighted instead of whichever row inherited the old flattened index.
#[test]
fn retained_session_identity_survives_an_authoritative_reorder() {
    let session = EntityId::from_bytes([31; 16]);
    let first = EntityId::from_bytes([32; 16]);
    let second = EntityId::from_bytes([33; 16]);
    let mut state = session_state(vec![
        session_row(SessionRowKind::Session, session, 0),
        session_row(SessionRowKind::Branch, first, 1),
        session_row(SessionRowKind::Branch, second, 1),
    ]);
    state.cursor = 2;
    state.session_cursor = Some(SessionRowIdentity {
        reference: second,
        kind: SessionRowKind::Branch,
    });

    // The stale numeric cursor still addresses index two, which now holds the
    // other semantic branch after the authoritative reorder.
    replace_session_rows(
        &mut state,
        vec![
            session_row(SessionRowKind::Session, session, 0),
            session_row(SessionRowKind::Branch, second, 1),
            session_row(SessionRowKind::Branch, first, 1),
        ],
    );

    let projection = projection(&state).expect("session tree projection");
    assert_eq!(
        projection.selected_id.as_deref(),
        Some(format!("session-row:Branch:{second}").as_str())
    );
}

/// Verifies a removed identity falls back to the first projected row and never
/// to the unrelated row that inherited the previous numeric position.
#[test]
fn removed_session_identity_falls_back_to_the_first_projected_row() {
    let session = EntityId::from_bytes([41; 16]);
    let removed = EntityId::from_bytes([42; 16]);
    let remaining = EntityId::from_bytes([43; 16]);
    let mut state = session_state(vec![
        session_row(SessionRowKind::Session, session, 0),
        session_row(SessionRowKind::Branch, remaining, 1),
    ]);
    state.cursor = 1;
    state.session_cursor = Some(SessionRowIdentity {
        reference: removed,
        kind: SessionRowKind::Branch,
    });

    let projection = projection(&state).expect("session tree projection");
    assert_eq!(projection.selected_id, None);

    let mut slot = None;
    sync_tree(&mut slot, &projection).expect("tree accepts the projection");
    assert_eq!(
        slot.as_ref()
            .expect("retained tree")
            .tree
            .selected_id()
            .as_deref(),
        Some(format!("session-row:Session:{session}").as_str())
    );
}

/// Verifies one retained upstream tree is reused across an item refresh, keeping
/// ScribeRig-driven collapse and selection state instead of rebuilding rows.
#[test]
fn retained_tree_reuses_collapse_and_selection_across_refreshes() {
    let session = EntityId::from_bytes([61; 16]);
    let parent = EntityId::from_bytes([62; 16]);
    let child = EntityId::from_bytes([63; 16]);
    let sibling = EntityId::from_bytes([64; 16]);
    let parent_key = format!("session-row:Branch:{parent}");
    let child_key = format!("session-row:Branch:{child}");
    let mut state = session_state(vec![
        session_row(SessionRowKind::Session, session, 0),
        session_row(SessionRowKind::Branch, parent, 1),
        session_row(SessionRowKind::Branch, child, 2),
    ]);
    state.session_cursor = Some(SessionRowIdentity {
        reference: parent,
        kind: SessionRowKind::Branch,
    });

    let before = projection(&state).expect("session tree projection");
    let mut slot = None;
    sync_tree(&mut slot, &before).expect("tree accepts the projection");
    let retained = slot.as_ref().expect("retained tree");
    assert!(
        retained
            .tree
            .visible_rows()
            .iter()
            .any(|row| row.id == child_key),
        "projected parents start expanded"
    );
    retained.tree.set_expanded(&parent_key, false);
    assert!(
        !retained
            .tree
            .visible_rows()
            .iter()
            .any(|row| row.id == child_key),
        "collapse must hide the child"
    );

    replace_session_rows(
        &mut state,
        vec![
            session_row(SessionRowKind::Session, session, 0),
            session_row(SessionRowKind::Branch, parent, 1),
            session_row(SessionRowKind::Branch, child, 2),
            session_row(SessionRowKind::Branch, sibling, 1),
        ],
    );
    let after = projection(&state).expect("session tree projection");
    assert_ne!(after.items, before.items);
    sync_tree(&mut slot, &after).expect("tree stays valid after the refresh");

    let retained = slot.as_ref().expect("retained tree");
    assert_eq!(
        retained.tree.selected_id().as_deref(),
        Some(parent_key.as_str()),
        "stable identity must survive the item refresh"
    );
    assert!(
        !retained
            .tree
            .visible_rows()
            .iter()
            .any(|row| row.id == child_key),
        "the retained tree must be reused, not rebuilt per projection"
    );
    assert!(
        retained
            .tree
            .visible_rows()
            .iter()
            .any(|row| row.id == format!("session-row:Branch:{sibling}")),
        "the newly projected row must appear"
    );
}
