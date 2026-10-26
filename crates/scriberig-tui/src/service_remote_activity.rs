//! Restores shared causal activity records from remote session projections.

use super::{ApplicationError, EntityId};
use scriberig_protocol::{SessionActivityItem, SessionActivityKind, SessionActivityState};
use scriberig_session::{
    Branch, CacheDisposition, ExecutionActivity, ModelInvocation, ModelOutput, ToolInvocation,
    ToolResult,
};

/// Restores safe activity references and records on a remote branch.
///
/// - `branch`: Branch being hydrated from the remote response.
/// - `items`: Ordered safe activity projections from the server.
///
/// Unit after activity records and references have been restored.
///
/// Returns an internal error when a relationship contains a malformed entity.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn restore_activity(
    branch: &mut Branch,
    items: Vec<SessionActivityItem>,
) -> Result<(), ApplicationError> {
    for item in items {
        restore_activity_item(branch, item)?;
    }
    Ok(())
}

/// Restores one safe activity record and its causal references.
///
/// - `branch`: Branch being hydrated from the remote response.
/// - `item`: Safe activity projection from the server.
///
/// Unit after the activity item has been restored.
///
/// Returns an internal error when a relationship contains a malformed entity.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn restore_activity_item(
    branch: &mut Branch,
    item: SessionActivityItem,
) -> Result<(), ApplicationError> {
    let id = activity_identity(&item)?;
    restore_activity_record(branch, item, id)
}

/// Resolves the durable identity used to restore one activity item.
///
/// - `item`: Safe activity projection from the server.
///
/// Parsed activity identity using durable references where required.
///
/// Returns an internal error when the activity identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn activity_identity(item: &SessionActivityItem) -> Result<EntityId, ApplicationError> {
    let durable_id = item.durable_ref.as_deref().unwrap_or(&item.id);
    match item.kind {
        SessionActivityKind::HumanInput => parse_activity_entity(durable_id),
        SessionActivityKind::AssistantOutput
            if item
                .parent
                .as_deref()
                .is_none_or(|parent| parent.ends_with(":human")) =>
        {
            parse_activity_entity(durable_id)
        }
        _ => parse_activity_entity(&item.id),
    }
}

/// Adds the projected activity record and relationship to the branch.
///
/// - `branch`: Branch being hydrated from the remote response.
/// - `item`: Safe activity projection from the server.
/// - `id`: Parsed identity for this activity item.
///
/// Unit after the activity record has been restored.
///
/// Returns an internal error when a relationship contains a malformed entity.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
#[allow(clippy::too_many_lines)] // One match keeps activity reconstruction ordered.
fn restore_activity_record(
    branch: &mut Branch,
    item: SessionActivityItem,
    id: EntityId,
) -> Result<(), ApplicationError> {
    match item.kind {
        SessionActivityKind::HumanInput => {
            branch.activity.push(ExecutionActivity::HumanInput(id));
        }
        SessionActivityKind::ModelInvocation => {
            branch.model_invocations.push(ModelInvocation {
                id,
                branch: branch.id,
                operation: item
                    .operation
                    .map(|value| {
                        value
                            .parse()
                            .map_err(|error| ApplicationError::InvalidRequest {
                                operation: "session activity".to_owned(),
                                message: format!("invalid model operation identity: {error}"),
                            })
                    })
                    .transpose()?,
                ordinal: item.sequence,
                agent_profile: None,
                context_fingerprint: None,
                provider_profile: None,
                provider_id: None,
                model: item.model,
                model_profile: None,
                provider_model_id: None,
                thinking_level: item.thinking_level,
                request_fingerprint: None,
                cache: CacheDisposition::NotUsed,
                output: None,
                request: None,
            });
            branch.activity.push(ExecutionActivity::ModelInvocation(id));
        }
        SessionActivityKind::AssistantOutput => {
            if let Some(parent) = item.parent.filter(|parent| !parent.ends_with(":human")) {
                let invocation = parse_activity_entity(&parent)?;
                branch.model_outputs.push(ModelOutput {
                    id,
                    invocation,
                    text: Some(item.summary),
                    structured_output: None,
                    finish_reason: None,
                    usage: None,
                });
                if let Some(record) = branch
                    .model_invocations
                    .iter_mut()
                    .find(|record| record.id == invocation)
                {
                    record.output = Some(id);
                }
                branch.activity.push(ExecutionActivity::ModelOutput(id));
            } else {
                branch.activity.push(ExecutionActivity::AgentOutput(id));
            }
        }
        SessionActivityKind::ToolInvocation => {
            let presentation_label = item.summary.clone();
            let parent_model_invocation = item
                .parent
                .as_deref()
                .map(parse_activity_entity)
                .transpose()?;
            branch.tool_invocations.push(ToolInvocation {
                id,
                name: item.tool_name.unwrap_or(item.summary),
                input: Vec::new(),
                parent_model_invocation,
                presentation_input: item
                    .tool_arguments
                    .map(|arguments| serde_json::to_vec(&arguments).unwrap_or_default()),
                presentation_label: Some(presentation_label),
                category: item.tool_category,
            });
            branch.activity.push(ExecutionActivity::ToolInvocation(id));
        }
        SessionActivityKind::ToolResult => {
            let invocation = item
                .parent
                .as_deref()
                .map(parse_activity_entity)
                .transpose()?
                .ok_or_else(|| malformed_activity("tool result has no invocation parent"))?;
            branch.tool_results.push(ToolResult {
                id,
                invocation,
                output: Vec::new(),
                failed: item.state == SessionActivityState::Failed,
                presentation_preview: item
                    .tool_result_preview
                    .and_then(|preview| serde_json::to_vec(&preview).ok()),
            });
            branch.activity.push(ExecutionActivity::ToolResult(id));
        }
        SessionActivityKind::BranchStarted => {
            if let Some(parent) = item.parent.as_deref() {
                branch.activity.push(ExecutionActivity::ChildBranchSpawned {
                    branch: id,
                    parent_tool_invocation: parse_activity_entity(parent)?,
                });
            } else {
                branch.activity.push(ExecutionActivity::ChildBranch(id));
            }
        }
        SessionActivityKind::BranchCompleted => {
            branch.activity.push(ExecutionActivity::BranchResult(id));
        }
        _ => {}
    }
    Ok(())
}

/// Parses a stable entity reference carried by activity.
///
/// - `value`: Remote activity identity.
///
/// Parsed entity identity.
///
/// Returns an internal error for malformed entity values.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_activity_entity(value: &str) -> Result<EntityId, ApplicationError> {
    value.parse().map_err(|error| ApplicationError::Internal {
        message: format!("remote activity contained invalid entity identity: {error}"),
    })
}

/// Creates a sanitized malformed-activity error.
///
/// - `message`: Safe validation detail.
///
/// An internal application error describing the invalid activity.
/// # Returns
/// The operation result.
fn malformed_activity(message: &str) -> ApplicationError {
    ApplicationError::Internal {
        message: message.to_owned(),
    }
}

#[cfg(test)]
#[path = "service_remote_activity_tests.rs"]
mod tests;
