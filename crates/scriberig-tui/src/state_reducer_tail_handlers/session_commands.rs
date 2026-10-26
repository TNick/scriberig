//! Applies local session list, switch, and removal results.

use super::*;

/// Opens a readable session list with stable IDs for follow-up commands.
///
/// - `state`: TUI state receiving the session list.
/// - `sessions`: Durable sessions returned by the application boundary.
///
/// A no-op effect after the list overlay is updated.
/// # Returns
/// The operation result.
pub(in crate::state::state_reducer_tail) fn sessions_listed(
    state: &mut TuiState,
    sessions: &[scriberig_session::Session],
) -> Vec<Effect> {
    let rows = sessions
        .iter()
        .map(|session| {
            let root = session
                .branches
                .iter()
                .find(|branch| branch.parent.is_none());
            let title = root.map_or("Conversation", |branch| branch.title.as_str());
            let status = root.map_or_else(
                || "empty".to_owned(),
                |branch| format!("{:?}", branch.status),
            );
            format!(
                "{} · {} · {} · {} branches",
                session.id,
                crate::inspector::sanitize(title),
                status,
                session.branches.len()
            )
        })
        .collect();
    state.overlay = Some(crate::overlay::Overlay::SessionList(rows));
    vec![Effect::None]
}

/// Loads one durable session as the active conversation and reply target.
///
/// - `state`: TUI state receiving the selected session.
/// - `session`: Full durable session graph returned by the application.
///
/// A no-op effect after replacing the active conversation projection.
/// # Returns
/// The operation result.
pub(in crate::state::state_reducer_tail) fn session_switched(
    state: &mut TuiState,
    session: scriberig_session::Session,
) -> Vec<Effect> {
    let Some(root) = session
        .branches
        .iter()
        .find(|branch| branch.parent.is_none())
    else {
        state.conversation_composer.error =
            Some("The selected session has no root branch.".to_owned());
        return vec![Effect::None];
    };
    let session_id = session.id;
    let root_branch = root.id;
    state.session = LoadState::Ready(crate::view::session::SessionModel::from_session(&session));
    state.conversation = Some(crate::state::conversation_projection(state, &session));
    state.session_snapshot = Some(session);
    state.conversation_cursor = Some(crate::view::conversation::ConversationSource::Session(
        session_id,
    ));
    state.cursor = 0;
    state.primary_view = crate::selection::PrimaryView::Conversation;
    state.focus = crate::state::Focus::Primary;
    state.conversation_browsing = false;
    state.conversation_composer.target = crate::state::ConversationTarget::Existing {
        session: session_id,
        reply_anchor: scriberig_session::ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
    };
    state.conversation_composer.pending = false;
    state.conversation_composer.pending_prompt = None;
    state.conversation_composer.error = None;
    state.overlay = None;
    state.follow_coordinator = false;
    state.coordinator_cursor = state
        .conversation
        .as_ref()
        .and_then(|conversation| conversation.coordinator_cursor);
    state.expanded_branches.clear();
    state
        .agent_handles
        .retain(|handle| handle.session != session_id);
    vec![Effect::None]
}

/// Clears active presentation when its durable session was removed.
///
/// - `state`: TUI state receiving the removal result.
/// - `session`: Stable identity of the removed session.
///
/// A no-op effect after clearing or preserving the current conversation.
/// # Returns
/// The operation result.
pub(in crate::state::state_reducer_tail) fn session_removed(
    state: &mut TuiState,
    session: scriberig_core::EntityId,
) -> Vec<Effect> {
    let active = state
        .session_snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.id == session);
    let targeted = matches!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::Existing {
            session: target,
            ..
        } if target == session
    );
    if active || targeted {
        state.session = LoadState::NotRequested;
        state.session_snapshot = None;
        state.conversation = None;
        state.conversation_cursor = None;
        state.conversation_composer.target = crate::state::ConversationTarget::NewSession;
        state.conversation_composer.pending = false;
        state.conversation_composer.pending_prompt = None;
        state.cursor = 0;
        state.coordinator_cursor = None;
        state.follow_coordinator = true;
        state.expanded_branches.clear();
        state
            .agent_handles
            .retain(|handle| handle.session != session);
    }
    state.conversation_composer.error = Some(format!("Removed session {session}."));
    state.overlay = None;
    vec![Effect::None]
}
