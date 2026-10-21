//! Identifies actions delegated to the session and operation reducer.

use crate::action::Action;

/// Reports whether the tail reducer owns an action variant.
///
/// - `action`: Action being classified before ordinary reduction.
///
/// Whether the action is handled by the tail reducer.
/// # Returns
/// `true` when the action belongs to the session/operation reducer.
pub(super) fn handles_tail_action(action: &Action) -> bool {
    matches!(
        action,
        Action::SessionSucceeded { .. }
            | Action::SessionsListed(_)
            | Action::SessionSwitched(_)
            | Action::SessionRemoved(_)
            | Action::PlanUpdated(_)
            | Action::AgentSubmitted(_)
            | Action::ConversationAgentAccepted(_)
            | Action::ConversationBrowsing(_)
            | Action::ConversationPinCursor
            | Action::ConversationFollowCoordinator
            | Action::ConversationToggleExpansion
            | Action::SessionRequested { .. }
            | Action::ComposerOpened { .. }
            | Action::SessionComposerOpened { .. }
            | Action::SessionComposerFocused(_)
            | Action::AgentOutputObserved { .. }
            | Action::CapabilitiesLoaded(_)
            | Action::OperationsLoaded(_)
            | Action::OperationObserved(_)
            | Action::OperationStatusRequested(_)
            | Action::OperationCancelRequested(_)
    )
}
