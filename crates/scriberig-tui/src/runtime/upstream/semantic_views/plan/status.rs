//! Maps durable plan statuses to upstream semantic theme roles.

use super::super::common::{SemanticRole, append_status};

/// Appends a durable plan lifecycle status using an upstream theme token.
///
/// - `output`: Markdown content rendered by the upstream component.
/// - `status`: Durable `ScribeRig` plan status.
pub(super) fn append_plan_status(output: &mut String, status: scriberig_session::PlanStatus) {
    let role = match status {
        scriberig_session::PlanStatus::Completed => SemanticRole::Success,
        scriberig_session::PlanStatus::Failed => SemanticRole::Error,
        scriberig_session::PlanStatus::Paused | scriberig_session::PlanStatus::Cancelled => {
            SemanticRole::Warning
        }
        scriberig_session::PlanStatus::Draft
        | scriberig_session::PlanStatus::Planned
        | scriberig_session::PlanStatus::Running => SemanticRole::Neutral,
    };
    append_status(output, "Status", &format!("{status:?}"), role);
}

/// Appends a durable plan-node status using an upstream theme token.
///
/// - `output`: Markdown content rendered by the upstream component.
/// - `label`: User-facing status field label.
/// - `status`: Durable `ScribeRig` plan-node status.
pub(super) fn append_node_status(
    output: &mut String,
    label: &str,
    status: scriberig_session::PlanNodeStatus,
) {
    let role = match status {
        scriberig_session::PlanNodeStatus::Completed => SemanticRole::Success,
        scriberig_session::PlanNodeStatus::Failed => SemanticRole::Error,
        scriberig_session::PlanNodeStatus::Abandoned => SemanticRole::Warning,
        scriberig_session::PlanNodeStatus::Planned
        | scriberig_session::PlanNodeStatus::Ready
        | scriberig_session::PlanNodeStatus::Running => SemanticRole::Neutral,
    };
    append_status(output, label, &format!("{status:?}"), role);
}
