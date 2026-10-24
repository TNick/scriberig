//! Shared row builders and projection assertions for the semantic tree tests.

use super::super::projection::TreeProjection;
use crate::inspector::LoadState;
use crate::state::TuiState;
use crate::view::conversation::ConversationModel;
use crate::view::session::{SessionModel, SessionRow, SessionRowKind};
use scriberig_core::EntityId;

/// Builds one bounded session tree row for projection assertions.
///
/// - `kind`: Public session object kind.
/// - `reference`: Stable semantic identity represented by the row.
/// - `depth`: Projected tree depth.
///
/// # Returns
/// A session row with a safe label.
pub(super) fn session_row(kind: SessionRowKind, reference: EntityId, depth: usize) -> SessionRow {
    SessionRow {
        reference,
        depth,
        label: format!("{kind:?} {reference}"),
        kind,
        candidate: None,
        snapshot: None,
        transient: false,
    }
}

/// Builds a ready Session view from the supplied semantic rows.
///
/// - `rows`: Ordered semantic session rows.
///
/// # Returns
/// Presentation state for the Session view.
pub(super) fn session_state(rows: Vec<SessionRow>) -> TuiState {
    let execution = scriberig_session::SessionExecutionSelection::default();
    TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: LoadState::Ready(SessionModel {
            rows,
            execution,
            tool_invocation_details: std::collections::BTreeMap::new(),
        }),
        ..TuiState::default()
    }
}

/// Replaces the Session view rows of an existing state.
///
/// - `state`: Presentation state receiving the authoritative rows.
/// - `rows`: New ordered semantic session rows.
pub(super) fn replace_session_rows(state: &mut TuiState, rows: Vec<SessionRow>) {
    if let LoadState::Ready(model) = &mut state.session {
        model.rows = rows;
    }
}

/// Builds a completed durable conversation with two answered turns.
///
/// - `session`: Durable session identity.
/// - `branch`: Root branch identity.
/// - `turns`: Turn identities in durable order.
///
/// A session whose root branch holds one answered turn per supplied identity.
/// # Returns
/// The canonical session value for conversation projection assertions.
pub(super) fn conversation_session(
    session: EntityId,
    branch: EntityId,
    turns: &[EntityId],
) -> scriberig_session::Session {
    let mut root = scriberig_session::Branch::new(branch, "Prior conversation").expect("root");
    for (ordinal, id) in turns.iter().copied().enumerate() {
        root.push_turn(scriberig_session::Turn {
            id,
            ordinal: u64::try_from(ordinal).expect("bounded fixture ordinal"),
            human: Some(scriberig_session::HumanInput {
                text: format!("question {ordinal}"),
            }),
            agent: Some(scriberig_session::AgentOutput {
                text: format!("answer {ordinal}"),
            }),
            tool_invocation: None,
            tool_result: None,
        });
    }
    root.status = scriberig_session::BranchStatus::Completed;
    scriberig_session::Session {
        id: session,
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    }
}

/// Applies an authoritative conversation refresh that relocates and shifts rows.
///
/// A completed historical continuation is anchored at the supplied turn, and an
/// unrelated sibling root branch is appended, so the continuation moves within
/// the projection while every later flattened position shifts.
///
/// - `session`: Canonical session to refresh in place.
/// - `branch`: Anchor branch identity.
/// - `turn`: Anchor turn identity for the historical continuation.
/// - `continuation`: New continuation branch identity.
/// - `sibling`: Unrelated root branch identity appended after the anchor tree.
pub(super) fn authoritative_conversation_refresh(
    session: &mut scriberig_session::Session,
    branch: EntityId,
    turn: EntityId,
    continuation: EntityId,
    sibling: EntityId,
) {
    session
        .add_continuation_branch(
            continuation,
            scriberig_session::ContinuationAnchor::AssistantOutput { branch, turn },
            "Historical reply",
            "continue the earlier answer",
            scriberig_session::ContinuationOrigin::HumanContinuation,
        )
        .expect("add historical continuation");
    let mut sibling =
        scriberig_session::Branch::new(sibling, "Unrelated sibling").expect("sibling");
    sibling.status = scriberig_session::BranchStatus::Completed;
    session.branches.push(sibling);
}

/// Builds Conversation presentation state with a retained durable turn cursor.
///
/// - `model`: Current conversation projection.
/// - `turn`: Durable turn identity retained by the conversation cursor.
///
/// # Returns
/// Presentation state for the Conversation view.
pub(super) fn conversation_state(model: ConversationModel, turn: EntityId) -> TuiState {
    TuiState {
        primary_view: crate::selection::PrimaryView::Conversation,
        conversation: Some(model),
        conversation_cursor: Some(crate::view::conversation::ConversationSource::Activity(
            turn,
        )),
        ..TuiState::default()
    }
}

/// Reports whether every projected upstream item ID is distinct.
///
/// - `projection`: Active semantic tree projection.
///
/// # Returns
/// `true` when upstream tree item validation can accept the projection.
pub(super) fn ids_are_unique(projection: &TreeProjection) -> bool {
    let mut ids = projection
        .items
        .iter()
        .map(|item| &item.id)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids.len() == projection.items.len()
}
