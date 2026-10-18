//! Restores shared causal activity records from remote session projections.

use super::{ApplicationError, EntityId};
use scriberig_protocol::{SessionActivityItem, SessionActivityKind, SessionActivityState};
use scriberig_session::{
    Branch, CacheDisposition, ExecutionActivity, ModelInvocation, ModelOutput, ToolInvocation,
    ToolResult,
};

/// Restores safe activity references and records on a remote branch.
///
/// # Parameters
/// - `branch`: Branch being hydrated from the remote response.
/// - `items`: Ordered safe activity projections from the server.
///
/// # Returns
/// Unit after activity records and references have been restored.
///
/// # Errors
/// Returns an internal error when a relationship contains a malformed entity.
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
/// # Parameters
/// - `branch`: Branch being hydrated from the remote response.
/// - `item`: Safe activity projection from the server.
///
/// # Returns
/// Unit after the activity item has been restored.
///
/// # Errors
/// Returns an internal error when a relationship contains a malformed entity.
fn restore_activity_item(
    branch: &mut Branch,
    item: SessionActivityItem,
) -> Result<(), ApplicationError> {
    let id = activity_identity(&item)?;
    restore_activity_record(branch, item, id)
}

/// Resolves the durable identity used to restore one activity item.
///
/// # Parameters
/// - `item`: Safe activity projection from the server.
///
/// # Returns
/// Parsed activity identity using durable references where required.
///
/// # Errors
/// Returns an internal error when the activity identity is malformed.
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
/// # Parameters
/// - `branch`: Branch being hydrated from the remote response.
/// - `item`: Safe activity projection from the server.
/// - `id`: Parsed identity for this activity item.
///
/// # Returns
/// Unit after the activity record has been restored.
///
/// # Errors
/// Returns an internal error when a relationship contains a malformed entity.
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
                ordinal: item.sequence,
                agent_profile: None,
                context_fingerprint: None,
                provider_profile: None,
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
        SessionActivityKind::ChildBranch => {
            if let Some(parent) = item.parent.as_deref() {
                branch.activity.push(ExecutionActivity::ChildBranchSpawned {
                    branch: id,
                    parent_tool_invocation: parse_activity_entity(parent)?,
                });
            } else {
                branch.activity.push(ExecutionActivity::ChildBranch(id));
            }
        }
        SessionActivityKind::BranchResult => {
            branch.activity.push(ExecutionActivity::BranchResult(id));
        }
    }
    Ok(())
}

/// Parses a stable entity reference carried by activity.
///
/// # Parameters
/// - `value`: Remote activity identity.
///
/// # Returns
/// Parsed entity identity.
///
/// # Errors
/// Returns an internal error for malformed entity values.
fn parse_activity_entity(value: &str) -> Result<EntityId, ApplicationError> {
    value.parse().map_err(|error| ApplicationError::Internal {
        message: format!("remote activity contained invalid entity identity: {error}"),
    })
}

/// Creates a sanitized malformed-activity error.
///
/// # Parameters
/// - `message`: Safe validation detail.
///
/// # Returns
/// An internal application error describing the invalid activity.
fn malformed_activity(message: &str) -> ApplicationError {
    ApplicationError::Internal {
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::restore_activity;
    use crate::view::session::SessionModel;
    use scriberig_core::EntityId;
    use scriberig_protocol::{SessionActivityItem, SessionActivityKind, SessionActivityState};
    use scriberig_session::{Branch, Session};

    /// Verifies remote hydration retains model, tool, result, and child causality.
    #[test]
    fn restored_tool_activity_keeps_model_parent_chain() {
        let model = EntityId::new();
        let tool = EntityId::new();
        let result = EntityId::new();
        let child = EntityId::new();
        let mut branch = Branch::new(EntityId::new(), "root").expect("branch");
        let branch_id = branch.id.to_string();
        restore_activity(
            &mut branch,
            vec![
                SessionActivityItem {
                    id: model.to_string(),
                    session: "session-1".to_owned(),
                    sequence: 0,
                    branch: branch_id.clone(),
                    parent: None,
                    kind: SessionActivityKind::ModelInvocation,
                    state: SessionActivityState::Completed,
                    summary: "Model request".to_owned(),
                    durable_ref: Some(model.to_string()),
                    model: Some("model-a".to_owned()),
                    thinking_level: None,
                    tool_name: None,
                    tool_arguments: None,
                    tool_result_preview: None,
                },
                SessionActivityItem {
                    id: tool.to_string(),
                    session: "session-1".to_owned(),
                    sequence: 1,
                    branch: branch_id.clone(),
                    parent: Some(model.to_string()),
                    kind: SessionActivityKind::ToolInvocation,
                    state: SessionActivityState::Completed,
                    summary: "Search semantic content".to_owned(),
                    durable_ref: Some(tool.to_string()),
                    model: None,
                    thinking_level: None,
                    tool_name: Some("search".to_owned()),
                    tool_arguments: None,
                    tool_result_preview: None,
                },
                SessionActivityItem {
                    id: result.to_string(),
                    session: "session-1".to_owned(),
                    sequence: 2,
                    branch: branch_id.clone(),
                    parent: Some(tool.to_string()),
                    kind: SessionActivityKind::ToolResult,
                    state: SessionActivityState::Completed,
                    summary: "Found one result".to_owned(),
                    durable_ref: Some(result.to_string()),
                    model: None,
                    thinking_level: None,
                    tool_name: None,
                    tool_arguments: None,
                    tool_result_preview: None,
                },
                SessionActivityItem {
                    id: child.to_string(),
                    session: "session-1".to_owned(),
                    sequence: 3,
                    branch: branch_id,
                    parent: Some(tool.to_string()),
                    kind: SessionActivityKind::ChildBranch,
                    state: SessionActivityState::Running,
                    summary: "Child branch".to_owned(),
                    durable_ref: Some(child.to_string()),
                    model: None,
                    thinking_level: None,
                    tool_name: None,
                    tool_arguments: None,
                    tool_result_preview: None,
                },
            ],
        )
        .expect("activity hydration");

        assert_eq!(
            branch.tool_invocations[0].parent_model_invocation,
            Some(model)
        );
        assert_eq!(
            branch.tool_invocations[0].presentation_label.as_deref(),
            Some("Search semantic content")
        );
        assert_eq!(branch.tool_results[0].invocation, tool);
        assert!(branch.activity.contains(
            &scriberig_session::ExecutionActivity::ChildBranchSpawned {
                branch: child,
                parent_tool_invocation: tool,
            }
        ));
    }

    /// Restores the shared cross-surface redaction fixture without changing IDs.
    #[test]
    fn shared_fixture_preserves_causal_activity_identity() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/agent_console_sensitive_activity.json"
        ))
        .expect("cross-surface fixture");
        let items = serde_json::from_value(fixture["protocolActivity"].clone())
            .expect("protocol activity items");
        let branch_id = EntityId::from_bytes([0x52; 16]);
        let mut branch = Branch::new(branch_id, "root").expect("branch");

        restore_activity(&mut branch, items).expect("restore shared activity");

        assert_eq!(
            branch.model_invocations[0].id,
            EntityId::from_bytes([0x53; 16])
        );
        assert_eq!(
            branch.tool_invocations[0].id,
            EntityId::from_bytes([0x54; 16])
        );
        assert_eq!(branch.tool_results[0].id, EntityId::from_bytes([0x55; 16]));
        assert_eq!(
            branch.tool_invocations[0].parent_model_invocation,
            Some(branch.model_invocations[0].id)
        );
        assert_eq!(
            branch.tool_results[0].invocation,
            branch.tool_invocations[0].id
        );
        assert_eq!(
            branch.tool_invocations[0].presentation_input.as_deref(),
            Some(br#"{"query":"find documents","token":"[REDACTED]"}"#.as_slice())
        );
        let session_id = EntityId::from_bytes([0x51; 16]);
        let session_model = SessionModel::from_session(&Session {
            id: session_id,
            branches: vec![branch],
            goals: Vec::new(),
            tasks: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        });
        let activity_ids = session_model
            .rows
            .iter()
            .map(|row| row.reference)
            .collect::<Vec<_>>();
        let mut state = crate::state::TuiState {
            primary_view: crate::selection::PrimaryView::Session,
            session: crate::inspector::LoadState::Ready(session_model),
            ..crate::state::TuiState::default()
        };
        let simple = crate::render::primary_text(&state);
        state.tool_invocation_visibility = crate::state::ToolInvocationVisibility::Detailed;
        let detailed = crate::render::primary_text(&state);

        assert!(simple.contains("Find documents"));
        assert!(detailed.contains("[REDACTED]"));
        assert_eq!(
            activity_ids,
            match &state.session {
                crate::inspector::LoadState::Ready(model) => model
                    .rows
                    .iter()
                    .map(|row| row.reference)
                    .collect::<Vec<_>>(),
                _ => unreachable!(),
            }
        );
    }
}
