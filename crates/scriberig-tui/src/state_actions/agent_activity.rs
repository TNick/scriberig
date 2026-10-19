//! Applies transient agent status to the TUI shell.

use crate::effect::Effect;
use crate::state::TuiState;

/// Displays a transient activity summary without changing durable session rows.
///
/// - `state`: TUI presentation state.
/// - `branch`: Runtime branch associated with this activity.
/// - `activity_kind`: Safe provider-neutral activity category.
/// - `activity_state`: Lifecycle state for this activity.
/// - `summary`: Safe runtime activity summary.
///
/// No asynchronous effects are required.
/// # Parameters
/// - `operation`: Input for this operation.
/// # Returns
/// The operation result.
pub(super) fn observe_agent_activity(
    state: &mut TuiState,
    operation: &str,
    branch: &str,
    activity_kind: scriberig_session::AgentActivityKind,
    activity_state: scriberig_session::AgentActivityState,
    summary: &str,
) -> Vec<Effect> {
    let key = stable_activity_key(operation, branch, activity_kind);
    let durable_selection = operation.starts_with("session-selection-");
    let completed_summary = activity_kind == scriberig_session::AgentActivityKind::ReasoningSummary
        && activity_state == scriberig_session::AgentActivityState::Completed;
    if activity_state != scriberig_session::AgentActivityState::Running
        && !durable_selection
        && !completed_summary
    {
        state
            .transient_agent_activities
            .retain(|activity| activity.key != key);
        if let Some(session) = state.session_snapshot.clone() {
            state.conversation = Some(crate::state::conversation_projection(state, &session));
        }
        return vec![Effect::None];
    }
    let activity = crate::state::TransientAgentActivity {
        key,
        branch: branch.chars().take(128).collect(),
        activity_kind,
        state: activity_state,
        summary: summary.chars().take(256).collect(),
    };
    if let Some(existing) = state
        .transient_agent_activities
        .iter_mut()
        .find(|existing| existing.key == key)
    {
        *existing = activity;
    } else {
        state.transient_agent_activities.push(activity);
        if state.transient_agent_activities.len() > 100 {
            state.transient_agent_activities.remove(0);
        }
    }
    if let Some(session) = state.session_snapshot.clone() {
        state.conversation = Some(crate::state::conversation_projection(state, &session));
    }
    if state.reasoning_visibility == crate::state::ReasoningVisibility::Hidden {
        return vec![Effect::None];
    }
    state.shell_message = match activity_kind {
        scriberig_session::AgentActivityKind::ReasoningSummary => {
            format!("Reasoning summary: {summary}")
        }
        _ => summary.to_owned(),
    };
    vec![Effect::None]
}

/// Derives a stable presentation identity from operation, branch, and category.
///
/// - `operation`: Shared operation identity.
/// - `branch`: Stable branch identity.
/// - `kind`: Public activity category.
///
/// A deterministic key independent of rendered row position.
/// # Returns
/// The operation result.
fn stable_activity_key(
    operation: &str,
    branch: &str,
    kind: scriberig_session::AgentActivityKind,
) -> u64 {
    let value = format!("{operation}\0{branch}\0{kind:?}");
    value.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::observe_agent_activity;
    use crate::state::{ReasoningVisibility, TuiState};

    /// Hides runtime and reasoning summaries until the user enables them.
    #[test]
    fn hidden_reasoning_preference_suppresses_status_rows() {
        let mut state = TuiState {
            shell_message: "unchanged".to_owned(),
            ..TuiState::default()
        };

        observe_agent_activity(
            &mut state,
            "operation-1",
            "branch-1",
            scriberig_session::AgentActivityKind::ReasoningSummary,
            scriberig_session::AgentActivityState::Completed,
            "safe summary",
        );
        observe_agent_activity(
            &mut state,
            "operation-1",
            "branch-1",
            scriberig_session::AgentActivityKind::RuntimeStatus,
            scriberig_session::AgentActivityState::Running,
            "running",
        );

        assert_eq!(state.shell_message, "unchanged");
        assert_eq!(state.transient_agent_activities.len(), 2);
    }

    /// Labels an explicitly user-facing summary separately from status.
    #[test]
    fn shown_reasoning_summary_is_labeled_distinctly() {
        let mut state = TuiState {
            reasoning_visibility: ReasoningVisibility::Shown,
            ..TuiState::default()
        };

        observe_agent_activity(
            &mut state,
            "operation-1",
            "branch-1",
            scriberig_session::AgentActivityKind::ReasoningSummary,
            scriberig_session::AgentActivityState::Completed,
            "safe summary",
        );

        assert_eq!(state.shell_message, "Reasoning summary: safe summary");
    }

    /// Updates one live operation row and clears it when the activity settles.
    #[test]
    fn repeated_activity_updates_one_stable_entry_then_clears() {
        let mut state = TuiState::default();
        observe_agent_activity(
            &mut state,
            "operation-2",
            "branch-2",
            scriberig_session::AgentActivityKind::RuntimeStatus,
            scriberig_session::AgentActivityState::Running,
            "starting",
        );
        observe_agent_activity(
            &mut state,
            "operation-2",
            "branch-2",
            scriberig_session::AgentActivityKind::RuntimeStatus,
            scriberig_session::AgentActivityState::Running,
            "working",
        );

        assert_eq!(state.transient_agent_activities.len(), 1);
        assert_eq!(state.transient_agent_activities[0].summary, "working");
        observe_agent_activity(
            &mut state,
            "operation-2",
            "branch-2",
            scriberig_session::AgentActivityKind::RuntimeStatus,
            scriberig_session::AgentActivityState::Completed,
            "completed",
        );
        assert_eq!(state.transient_agent_activities, []);
    }
}
