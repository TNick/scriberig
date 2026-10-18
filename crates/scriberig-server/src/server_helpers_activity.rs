//! Projects safe durable execution activity for server clients.

#[path = "server_helpers_activity/tool_presentation.rs"]
mod tool_presentation;
use scriberig_core::EntityId;
use scriberig_protocol::{
    SessionActivityItem, SessionActivityKind as Kind, SessionActivityState as State,
};
use scriberig_session::{Branch, BranchStatus, ExecutionActivity as Activity};
use tool_presentation::{
    activity_tool_details, bounded_summary, projected_activity_id, tool_result_summary,
};

/// Projects ordered execution references without exposing raw tool payloads.
///
/// # Parameters
/// - `session`: Stable identity of the containing session.
/// - `branch`: Durable branch containing execution records.
///
/// # Returns
/// Safe activity items in the branch's recorded causal order.
pub(super) fn branch_execution_activity(
    session: &str,
    branch: &Branch,
) -> Vec<SessionActivityItem> {
    branch
        .activity
        .iter()
        .enumerate()
        .filter_map(|(sequence, activity)| project_item(session, branch, sequence, activity))
        .collect()
}

/// Contains one safe projection of durable branch activity.
///
/// Fields retain only display-safe summaries and stable causal references.
struct ActivityProjection {
    /// Durable activity identity.
    id: EntityId,
    /// Optional parent activity identity.
    parent: Option<String>,
    /// Provider-neutral activity kind.
    kind: Kind,
    /// Current lifecycle state.
    state: State,
    /// Bounded display summary.
    summary: String,
    /// Selected model identifier, when this is a model request.
    model: Option<String>,
    /// Selected thinking level, when this is a model request.
    thinking_level: Option<String>,
    /// Public tool name, when this is a tool invocation.
    tool_name: Option<String>,
}

/// Projects one ordered activity into its public protocol representation.
///
/// # Parameters
/// - `session`: Stable session identity.
/// - `branch`: Durable branch containing the activity.
/// - `sequence`: Position in the durable causal activity list.
/// - `activity`: Durable activity reference to project.
///
/// # Returns
/// Safe protocol activity with stable parent and durable references.
fn project_item(
    session: &str,
    branch: &Branch,
    sequence: usize,
    activity: &Activity,
) -> Option<SessionActivityItem> {
    let projection = activity_projection(activity, branch)?;
    let (tool_arguments, tool_result_preview) = activity_tool_details(activity, branch);
    let id = projected_activity_id(projection.kind, projection.parent.is_none(), projection.id);
    let parent = match (projection.kind, projection.parent) {
        (Kind::AssistantOutput, None) => Some(format!("{}:human", projection.id)),
        (_, parent) => parent,
    };
    Some(SessionActivityItem {
        id,
        session: session.to_owned(),
        sequence: u64::try_from(sequence).unwrap_or(u64::MAX),
        branch: branch.id.to_string(),
        parent,
        kind: projection.kind,
        state: projection.state,
        summary: bounded_summary(&projection.summary),
        durable_ref: Some(projection.id.to_string()),
        model: projection.model,
        thinking_level: projection.thinking_level,
        tool_name: projection.tool_name,
        tool_arguments,
        tool_result_preview,
    })
}

/// Converts one durable activity reference to display-safe activity fields.
///
/// # Parameters
/// - `activity`: Durable activity reference.
/// - `branch`: Branch containing referenced records.
///
/// # Returns
/// Safe activity fields when the referenced durable record exists.
fn activity_projection(activity: &Activity, branch: &Branch) -> Option<ActivityProjection> {
    match activity {
        Activity::ModelInvocation(id) => project_model_invocation(*id, branch),
        Activity::HumanInput(id) => project_human_input(*id, branch),
        Activity::ModelOutput(id) | Activity::AgentOutput(id) => {
            project_assistant_output(*id, branch)
        }
        Activity::ToolInvocation(id) => project_tool_invocation(*id, branch),
        Activity::ToolResult(id) => project_tool_result(*id, branch),
        Activity::ChildBranch(id) => Some(child_branch_projection(*id, None)),
        Activity::ChildBranchSpawned {
            branch,
            parent_tool_invocation,
        } => Some(child_branch_projection(
            *branch,
            Some(*parent_tool_invocation),
        )),
        Activity::BranchResult(id) => project_branch_result(*id, branch),
    }
}

/// Projects one model invocation and its selected model metadata.
///
/// # Parameters
/// - `id`: Durable model-invocation identity.
/// - `branch`: Branch containing model invocation records.
///
/// # Returns
/// Safe model invocation fields when the record exists.
fn project_model_invocation(id: EntityId, branch: &Branch) -> Option<ActivityProjection> {
    let invocation = branch.model_invocations.iter().find(|item| item.id == id)?;
    let model = invocation.model.clone();
    let summary = model.as_ref().map_or_else(
        || "Model request".to_owned(),
        |value| format!("Model request: {value}"),
    );
    Some(ActivityProjection {
        id,
        parent: None,
        kind: Kind::ModelInvocation,
        state: model_invocation_state(branch.status, invocation.output.is_some()),
        summary,
        model,
        thinking_level: invocation.thinking_level.clone(),
        tool_name: None,
    })
}

/// Resolves model invocation lifecycle state from branch and output records.
///
/// # Parameters
/// - `branch`: Current lifecycle state of the branch.
/// - `has_output`: Whether an output record already exists.
///
/// # Returns
/// Public model invocation lifecycle state.
fn model_invocation_state(branch: BranchStatus, has_output: bool) -> State {
    match branch {
        BranchStatus::Completed => State::Completed,
        BranchStatus::Active if has_output => State::Completed,
        BranchStatus::Active => State::Running,
        BranchStatus::Abandoned => State::Failed,
    }
}

/// Projects one human-input marker without exposing prompt content.
///
/// # Parameters
/// - `id`: Durable turn identity.
/// - `branch`: Branch containing the input turn.
///
/// # Returns
/// Safe human-input marker when the turn exists.
fn project_human_input(id: EntityId, branch: &Branch) -> Option<ActivityProjection> {
    branch
        .turns
        .iter()
        .any(|turn| turn.id == id)
        .then(|| ActivityProjection {
            id,
            parent: None,
            kind: Kind::HumanInput,
            state: State::Completed,
            summary: "Human input".to_owned(),
            model: None,
            thinking_level: None,
            tool_name: None,
        })
}

/// Projects a model or agent output summary without the raw input record.
///
/// # Parameters
/// - `id`: Durable output or turn identity.
/// - `branch`: Branch containing output records.
///
/// # Returns
/// Safe assistant-output fields when the record exists.
fn project_assistant_output(id: EntityId, branch: &Branch) -> Option<ActivityProjection> {
    let (parent, summary) =
        if let Some(output) = branch.model_outputs.iter().find(|item| item.id == id) {
            (
                Some(output.invocation.to_string()),
                output.text.as_ref().map_or_else(
                    || "Assistant output".to_owned(),
                    |value| bounded_summary(value),
                ),
            )
        } else {
            let turn = branch.turns.iter().find(|turn| turn.id == id)?;
            (None, bounded_summary(&turn.agent.as_ref()?.text))
        };
    Some(ActivityProjection {
        id,
        parent,
        kind: Kind::AssistantOutput,
        state: State::Completed,
        summary,
        model: None,
        thinking_level: None,
        tool_name: None,
    })
}

/// Projects a tool invocation using its safe presentation label.
///
/// # Parameters
/// - `id`: Durable tool-invocation identity.
/// - `branch`: Branch containing invocation and result records.
///
/// # Returns
/// Safe tool invocation fields when the record exists.
fn project_tool_invocation(id: EntityId, branch: &Branch) -> Option<ActivityProjection> {
    let invocation = branch.tool_invocations.iter().find(|item| item.id == id)?;
    let has_result = branch
        .tool_results
        .iter()
        .any(|result| result.invocation == invocation.id);
    Some(ActivityProjection {
        id,
        parent: invocation
            .parent_model_invocation
            .map(|parent| parent.to_string()),
        kind: Kind::ToolInvocation,
        state: if has_result {
            State::Completed
        } else {
            State::Running
        },
        summary: invocation
            .presentation_label
            .clone()
            .unwrap_or_else(|| format!("Tool: {}", invocation.name)),
        model: None,
        thinking_level: None,
        tool_name: Some(invocation.name.clone()),
    })
}

/// Projects a tool result without serializing raw output bytes.
///
/// # Parameters
/// - `id`: Durable tool-result identity.
/// - `branch`: Branch containing the result record.
///
/// # Returns
/// Safe bounded tool-result fields when the record exists.
fn project_tool_result(id: EntityId, branch: &Branch) -> Option<ActivityProjection> {
    let result = branch.tool_results.iter().find(|item| item.id == id)?;
    Some(ActivityProjection {
        id,
        parent: Some(result.invocation.to_string()),
        kind: Kind::ToolResult,
        state: if result.failed {
            State::Failed
        } else {
            State::Completed
        },
        summary: tool_result_summary(result, result.failed),
        model: None,
        thinking_level: None,
        tool_name: None,
    })
}

/// Projects a child-branch reference and optional spawning tool parent.
///
/// # Parameters
/// - `id`: Child branch identity.
/// - `parent`: Optional spawning tool invocation identity.
///
/// # Returns
/// Safe child-branch fields.
fn child_branch_projection(id: EntityId, parent: Option<EntityId>) -> ActivityProjection {
    ActivityProjection {
        id,
        parent: parent.map(|parent| parent.to_string()),
        kind: Kind::ChildBranch,
        state: State::Running,
        summary: "Child branch".to_owned(),
        model: None,
        thinking_level: None,
        tool_name: None,
    }
}

/// Projects a branch result and its terminal status.
///
/// # Parameters
/// - `id`: Durable branch-result identity.
/// - `branch`: Branch containing the result.
///
/// # Returns
/// Safe branch-result fields when the result exists.
fn project_branch_result(id: EntityId, branch: &Branch) -> Option<ActivityProjection> {
    let result = branch.result.as_ref().filter(|result| result.id == id)?;
    Some(ActivityProjection {
        id,
        parent: None,
        kind: Kind::BranchResult,
        state: match result.status {
            BranchStatus::Completed => State::Completed,
            BranchStatus::Active => State::Running,
            BranchStatus::Abandoned => State::Failed,
        },
        summary: result
            .summary
            .as_deref()
            .map_or_else(|| "Branch result".to_owned(), bounded_summary),
        model: None,
        thinking_level: None,
        tool_name: None,
    })
}

#[cfg(test)]
#[path = "server_helpers_activity_oversized_tests.rs"]
mod oversized_tests;
#[cfg(test)]
#[path = "server_helpers_activity_tests.rs"]
mod tests;
