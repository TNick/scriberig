//! Projects safe runtime events into transient public activity.

use crate::orchestration::{BranchPhase, RuntimeEvent};

/// Converts one normalized runtime event into public activity fields.
///
/// # Parameters
/// - `event`: Safe normalized branch event.
///
/// # Returns
/// Public kind, state, and summary for displayable lifecycle activity.
pub(super) fn project_runtime_activity(
    event: &RuntimeEvent,
) -> Option<(&'static str, &'static str, String)> {
    match event {
        RuntimeEvent::PhaseChanged { phase, .. } => Some((
            "status",
            match phase {
                BranchPhase::Completed => "completed",
                BranchPhase::Failed | BranchPhase::BudgetExceeded => "failed",
                BranchPhase::Cancelled => "cancelled",
                _ => "running",
            },
            format!("Phase: {phase:?}"),
        )),
        RuntimeEvent::ModelInvocationStarted { step, .. } => Some((
            "modelInvocation",
            "running",
            format!("Model request {step}"),
        )),
        RuntimeEvent::ReasoningSummary { summary, .. } => Some((
            "reasoningSummary",
            "completed",
            summary.chars().take(256).collect(),
        )),
        RuntimeEvent::ToolStarted { tool, .. } => {
            Some(("toolInvocation", "running", format!("Tool: {tool}")))
        }
        RuntimeEvent::ToolCompleted { tool, success, .. } => Some((
            "toolResult",
            if *success { "completed" } else { "failed" },
            format!(
                "Tool {tool} {}",
                if *success { "completed" } else { "failed" }
            ),
        )),
        RuntimeEvent::BranchResultProduced { summary, .. } => {
            Some(("branchResult", "completed", summary.clone()))
        }
        RuntimeEvent::BranchExecutionFailed { category, .. } => Some((
            "branch",
            "failed",
            format!("Agent branch failed ({category})"),
        )),
        RuntimeEvent::BranchExecutionCancelled { .. } => {
            Some(("branch", "cancelled", "Agent branch cancelled".to_owned()))
        }
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
            Some(("status", "failed", "Phase: Failed".to_owned()))
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
                "reasoningSummary",
                "completed",
                "I will compare the two revisions.".to_owned()
            ))
        );
    }
}
