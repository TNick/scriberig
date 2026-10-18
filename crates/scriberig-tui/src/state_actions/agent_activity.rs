//! Applies transient agent status to the TUI shell.

use crate::effect::Effect;
use crate::state::TuiState;

/// Displays a transient activity summary without changing durable session rows.
///
/// # Parameters
/// - `state`: TUI presentation state.
/// - `branch`: Runtime branch associated with this activity.
/// - `activity_kind`: Safe provider-neutral activity category.
/// - `activity_state`: Lifecycle state for this activity.
/// - `summary`: Safe runtime activity summary.
///
/// # Returns
/// No asynchronous effects are required.
pub(super) fn observe_agent_activity(
    state: &mut TuiState,
    branch: &str,
    activity_kind: &str,
    activity_state: &str,
    summary: &str,
) -> Vec<Effect> {
    let activity = crate::state::TransientAgentActivity {
        branch: branch.chars().take(128).collect(),
        activity_kind: activity_kind.chars().take(64).collect(),
        state: activity_state.chars().take(32).collect(),
        summary: summary.chars().take(256).collect(),
    };
    if state.transient_agent_activities.last() != Some(&activity) {
        state.transient_agent_activities.push(activity);
        if state.transient_agent_activities.len() > 100 {
            state.transient_agent_activities.remove(0);
        }
    }
    if state.reasoning_visibility == crate::state::ReasoningVisibility::Hidden {
        return vec![Effect::None];
    }
    state.shell_message = match activity_kind {
        "reasoningSummary" => format!("Reasoning summary: {summary}"),
        _ => summary.to_owned(),
    };
    vec![Effect::None]
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
            "branch-1",
            "reasoningSummary",
            "completed",
            "safe summary",
        );
        observe_agent_activity(&mut state, "branch-1", "status", "running", "running");

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
            "branch-1",
            "reasoningSummary",
            "completed",
            "safe summary",
        );

        assert_eq!(state.shell_message, "Reasoning summary: safe summary");
    }
}
