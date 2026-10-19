//! Projects safe runtime events into transient public activity.

use crate::orchestration::{BranchPhase, RuntimeEvent};

/// Converts one normalized runtime event into public activity fields.
///
/// - `event`: Safe normalized branch event.
///
/// Public kind, state, and summary for displayable lifecycle activity.
/// # Returns
/// The operation result.
pub(super) fn project_runtime_activity(
    event: &RuntimeEvent,
) -> Option<(
    scriberig_session::AgentActivityKind,
    scriberig_session::AgentActivityState,
    String,
)> {
    match event {
        RuntimeEvent::PhaseChanged { phase, .. } => Some((
            scriberig_session::AgentActivityKind::RuntimeStatus,
            match phase {
                BranchPhase::Completed => scriberig_session::AgentActivityState::Completed,
                BranchPhase::Failed | BranchPhase::BudgetExceeded => {
                    scriberig_session::AgentActivityState::Failed
                }
                BranchPhase::Cancelled => scriberig_session::AgentActivityState::Cancelled,
                _ => scriberig_session::AgentActivityState::Running,
            },
            format!("Phase: {phase:?}"),
        )),
        RuntimeEvent::ModelInvocationStarted { step, .. } => Some((
            scriberig_session::AgentActivityKind::ModelInvocation,
            scriberig_session::AgentActivityState::Running,
            format!("Model request {step}"),
        )),
        RuntimeEvent::ReasoningSummary { summary, .. } => Some((
            scriberig_session::AgentActivityKind::ReasoningSummary,
            scriberig_session::AgentActivityState::Completed,
            summary.chars().take(256).collect(),
        )),
        RuntimeEvent::ToolStarted { tool, .. } => Some((
            scriberig_session::AgentActivityKind::ToolInvocation,
            scriberig_session::AgentActivityState::Running,
            format!("Tool: {tool}"),
        )),
        RuntimeEvent::ToolCompleted { tool, success, .. } => Some((
            scriberig_session::AgentActivityKind::ToolResult,
            if *success {
                scriberig_session::AgentActivityState::Completed
            } else {
                scriberig_session::AgentActivityState::Failed
            },
            format!(
                "Tool {tool} {}",
                if *success { "completed" } else { "failed" }
            ),
        )),
        RuntimeEvent::BranchResultProduced { summary, .. } => Some((
            scriberig_session::AgentActivityKind::BranchCompleted,
            scriberig_session::AgentActivityState::Completed,
            summary.clone(),
        )),
        RuntimeEvent::BranchExecutionFailed { category, .. } => Some((
            scriberig_session::AgentActivityKind::Error,
            scriberig_session::AgentActivityState::Failed,
            format!("Agent branch failed ({category})"),
        )),
        RuntimeEvent::BranchExecutionCancelled { .. } => Some((
            scriberig_session::AgentActivityKind::BranchCompleted,
            scriberig_session::AgentActivityState::Cancelled,
            "Agent branch cancelled".to_owned(),
        )),
        RuntimeEvent::BranchExecutionStarted { .. }
        | RuntimeEvent::ModelTextDelta { .. }
        | RuntimeEvent::ChildBranchStarted { .. }
        | RuntimeEvent::ChildBranchCompleted { .. }
        | RuntimeEvent::BranchExecutionCompleted { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::project_runtime_activity;
    use crate::orchestration::{BranchPhase, RuntimeEvent};

    /// Keeps streamed provider text out of status activity summaries.
    #[test]
    fn model_text_delta_is_not_projected_as_runtime_activity() {
        let event = RuntimeEvent::ModelTextDelta {
            branch: "branch-1".to_owned(),
            text: "unclassified provider text".to_owned(),
        };

        assert!(project_runtime_activity(&event).is_none());
    }

    /// Preserves failure status when projecting a terminal runtime phase.
    #[test]
    fn failed_phase_projects_a_failed_state() {
        let event = RuntimeEvent::PhaseChanged {
            branch: "branch-1".to_owned(),
            phase: BranchPhase::Failed,
        };

        assert_eq!(
            project_runtime_activity(&event),
            Some((
                scriberig_session::AgentActivityKind::RuntimeStatus,
                scriberig_session::AgentActivityState::Failed,
                "Phase: Failed".to_owned()
            ))
        );
    }

    /// Keeps explicitly user-facing reasoning distinct from runtime status.
    #[test]
    fn reasoning_summary_has_its_own_activity_kind() {
        let event = RuntimeEvent::ReasoningSummary {
            branch: "branch-1".to_owned(),
            summary: "I will compare the two revisions.".to_owned(),
        };

        assert_eq!(
            project_runtime_activity(&event),
            Some((
                scriberig_session::AgentActivityKind::ReasoningSummary,
                scriberig_session::AgentActivityState::Completed,
                "I will compare the two revisions.".to_owned()
            ))
        );
    }
}
