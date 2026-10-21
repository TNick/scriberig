//! Builds safe supplementary detail text for semantic tree projections.

use crate::state::TuiState;

#[path = "details/plan_graph.rs"]
mod plan_graph;

/// Builds request and live details that accompany conversation tree rows.
///
/// - `state`: Current `ScribeRig` conversation and plan projections.
///
/// Bounded supplementary content not contained in tree row labels.
/// # Returns
/// Sanitized Markdown content for the active conversation.
pub(super) fn conversation_details(state: &TuiState) -> String {
    let mut output = String::from("# Conversation details\n\n");
    if state.prompt_visibility == crate::state::PromptVisibility::Shown
        && let Some(model) = &state.conversation
    {
        for row in &model.rows {
            if let crate::view::conversation::ConversationSource::Activity(invocation) = row.source
                && let Some(detail) = state.model_request_details.get(&invocation)
            {
                append_model_request(&mut output, detail);
            }
        }
    }
    if state.reasoning_visibility == crate::state::ReasoningVisibility::Shown {
        for activity in state
            .transient_agent_activities
            .iter()
            .take(super::common::MAX_ROWS)
        {
            super::common::append_value(
                &mut output,
                &format!(
                    "{} · {:?}: {}",
                    activity.branch, activity.state, activity.summary
                ),
            );
        }
    }
    for response in state
        .transient_agent_outputs
        .iter()
        .take(super::common::MAX_ROWS)
    {
        super::common::append_value(&mut output, &response.text);
    }
    if let Some(prompt) = state.conversation_composer.pending_prompt.as_deref() {
        super::common::append_value(&mut output, &format!("You: {prompt}"));
    }
    if let Some(error) = state.conversation_composer.error.as_deref() {
        super::common::append_value(&mut output, &format!("Error: {error}"));
    }
    plan_graph::append_plan_graphs(&mut output, state);
    output
}

/// Builds session execution details alongside the upstream session tree.
///
/// - `state`: Current `ScribeRig` session read model.
///
/// Safe model assignment display data.
/// # Returns
/// Sanitized Markdown content for the active session.
pub(super) fn session_details(state: &TuiState) -> String {
    let mut output = String::from("# Session details\n\n");
    if let crate::inspector::LoadState::Ready(model) = &state.session {
        super::common::append_value(
            &mut output,
            &format!(
                "Session execution: {} / {}",
                model
                    .execution
                    .provider_id
                    .as_deref()
                    .unwrap_or("unassigned"),
                model.execution.model_id.as_deref().unwrap_or("unassigned")
            ),
        );
        for change in model.execution.history.iter().take(super::common::MAX_ROWS) {
            super::common::append_value(
                &mut output,
                &format!(
                    "{:?} selection · revision {}: {}",
                    change.kind, change.revision, change.summary
                ),
            );
        }
        if let Some(row) = model.rows.get(state.cursor) {
            append_selected_session_activity(&mut output, state, model, row);
        }
    }
    output
}

/// Appends the selected session activity's permitted cached details.
///
/// - `output`: Markdown receiving safe selected-row details.
/// - `state`: Client-local presentation visibility settings and cached details.
/// - `model`: Exact `ScribeRig` session projection containing the selected row.
/// - `row`: Selected activity row from the projection.
fn append_selected_session_activity(
    output: &mut String,
    state: &TuiState,
    model: &crate::view::session::SessionModel,
    row: &crate::view::session::SessionRow,
) {
    super::common::append_value(output, &format!("Selected activity: {}", row.label));
    match row.kind {
        crate::view::session::SessionRowKind::ModelInvocation => {
            if state.prompt_visibility == crate::state::PromptVisibility::Shown
                && let Some(detail) = state.model_request_details.get(&row.reference)
            {
                append_model_request(output, detail);
            }
        }
        crate::view::session::SessionRowKind::ToolInvocation => {
            if state.tool_invocation_visibility != crate::state::ToolInvocationVisibility::Hidden
                && let Some(detail) = model.tool_invocation_details.get(&row.reference)
            {
                super::common::append_value(output, &format!("Safe tool arguments: {detail}"));
            }
        }
        crate::view::session::SessionRowKind::ToolResult => match state.tool_output_visibility {
            crate::state::ToolOutputVisibility::Hidden => {
                super::common::append_value(output, "Tool result hidden.");
            }
            crate::state::ToolOutputVisibility::Summary => {}
            crate::state::ToolOutputVisibility::Full => {
                if let Some(detail) = state.tool_result_details.get(&row.reference) {
                    let content = serde_json::to_string_pretty(&detail.detail)
                        .unwrap_or_else(|_| "{}".to_owned());
                    super::common::append_value(output, &format!("Tool result detail: {content}"));
                }
            }
        },
        _ => {}
    }
}

/// Appends the safe request projection associated with one invocation.
///
/// - `output`: Markdown content receiving request details.
/// - `detail`: Bounded, credential-safe request projection.
fn append_model_request(
    output: &mut String,
    detail: &scriberig_protocol::ModelRequestDetailResponse,
) {
    super::common::append_value(
        output,
        &format!(
            "Effective request · {} · {}",
            detail.invocation, detail.model
        ),
    );
    if let Some(provider) = &detail.provider_id {
        super::common::append_value(output, &format!("Provider: {provider}"));
    }
    if let Some(thinking) = &detail.thinking_level {
        super::common::append_value(output, &format!("Thinking level: {thinking}"));
    }
    if let Some(profile) = &detail.profile {
        super::common::append_value(
            output,
            &format!(
                "Profile revision: {} @ {}",
                profile.identity, profile.revision
            ),
        );
    }
    for component in detail.active_components.iter().take(64) {
        super::common::append_value(
            output,
            &format!(
                "Context component: {} @ {}",
                component.identity, component.revision
            ),
        );
    }
    for message in detail.messages.iter().take(64) {
        super::common::append_value(output, &format!("{}: {}", message.role, message.content));
    }
    for tool in detail.tools.iter().take(64) {
        super::common::append_value(
            output,
            &format!("Tool: {} · {}", tool.name, tool.description),
        );
        if let Some(parameters) = &tool.parameters {
            super::common::append_value(output, &format!("Tool parameters: {parameters}"));
        }
    }
    if let Some(report) = &detail.applied_settings {
        let summary = serde_json::to_string(report).unwrap_or_else(|_| "{}".to_owned());
        super::common::append_value(output, &format!("Applied settings: {summary}"));
    }
    if detail.truncated {
        super::common::append_value(output, "Effective request detail is truncated.");
    }
}

#[cfg(test)]
mod tests {
    use super::{conversation_details, session_details};
    use crate::inspector::LoadState;
    use crate::state::{
        PromptVisibility, ReasoningVisibility, ToolInvocationVisibility, ToolOutputVisibility,
        TuiState,
    };
    use crate::view::conversation::{
        ConversationModel, ConversationRow, ConversationRowKind, ConversationSource,
    };
    use crate::view::session::{SessionModel, SessionRow, SessionRowKind};
    use scriberig_core::EntityId;

    /// Uses only redacted cached arguments for the selected tool invocation.
    #[test]
    fn session_details_obey_tool_visibility_and_keep_redaction() {
        let invocation = EntityId::from_bytes([1; 16]);
        let model = SessionModel {
            rows: vec![SessionRow {
                reference: invocation,
                depth: 0,
                label: "Find documents".to_owned(),
                kind: SessionRowKind::ToolInvocation,
                candidate: None,
                snapshot: None,
                transient: false,
            }],
            execution: scriberig_session::SessionExecutionSelection {
                revision: 2,
                provider_id: Some("provider-b".to_owned()),
                model_id: Some("model-b".to_owned()),
                thinking_level: Some("careful".to_owned()),
                history: vec![scriberig_session::SessionExecutionChange {
                    revision: 2,
                    kind: scriberig_session::AgentActivityKind::ThinkingLevelChanged,
                    summary: "Thinking level selected: careful".to_owned(),
                    provider_id: Some("provider-b".to_owned()),
                    model_id: Some("model-b".to_owned()),
                    thinking_level: Some("careful".to_owned()),
                }],
            },
            tool_invocation_details: [(invocation, "{\"token\":\"[REDACTED]\"}".to_owned())]
                .into_iter()
                .collect(),
        };
        let mut state = TuiState {
            session: LoadState::Ready(model),
            ..TuiState::default()
        };

        let hidden = session_details(&state);
        assert!(!hidden.contains("[REDACTED]"));
        assert!(hidden.contains("Thinking level selected: careful"));
        for visibility in [
            ToolInvocationVisibility::CompactTruncated,
            ToolInvocationVisibility::Full,
            ToolInvocationVisibility::StructuredParameters,
        ] {
            state.tool_invocation_visibility = visibility;
            let visible = session_details(&state);
            assert!(visible.contains("[REDACTED]"));
            assert!(!visible.contains("SECRET_MARKER"));
        }
    }

    /// Shows cached request and reasoning only after their local controls enable them.
    #[test]
    fn conversation_details_follow_prompt_and_reasoning_visibility() {
        let invocation = EntityId::from_bytes([2; 16]);
        let mut state = TuiState {
            conversation: Some(ConversationModel {
                rows: vec![ConversationRow {
                    source: ConversationSource::Activity(invocation),
                    reply_anchor: None,
                    depth: 0,
                    label: "Model invocation".to_owned(),
                    kind: ConversationRowKind::Activity,
                }],
                coordinator_cursor: None,
            }),
            model_request_details: [(
                invocation,
                scriberig_protocol::ModelRequestDetailResponse {
                    invocation: invocation.to_string(),
                    model: "model-a".to_owned(),
                    provider_id: None,
                    thinking_level: None,
                    profile: None,
                    active_components: Vec::new(),
                    messages: vec![scriberig_protocol::ModelRequestMessageDetail {
                        role: "user".to_owned(),
                        content: "Safe captured request".to_owned(),
                    }],
                    tools: Vec::new(),
                    applied_settings: None,
                    applied_settings_fingerprint: None,
                    truncated: false,
                },
            )]
            .into_iter()
            .collect(),
            transient_agent_activities: vec![crate::state::TransientAgentActivity {
                key: 1,
                sequence: 1,
                branch: "branch-1".to_owned(),
                activity_kind: scriberig_session::AgentActivityKind::ReasoningSummary,
                state: scriberig_session::AgentActivityState::Completed,
                summary: "Compare the selected revisions.".to_owned(),
            }],
            ..TuiState::default()
        };

        let hidden = conversation_details(&state);
        assert!(!hidden.contains("Safe captured request"));
        assert!(!hidden.contains("Compare the selected revisions."));
        state.prompt_visibility = PromptVisibility::Shown;
        state.reasoning_visibility = ReasoningVisibility::Shown;
        let visible = conversation_details(&state);
        assert!(visible.contains("Safe captured request"));
        assert!(visible.contains("Compare the selected revisions."));
        assert_eq!(state.transient_agent_activities.len(), 1);
    }

    /// Hides or exposes tool-result detail according to its local display mode.
    #[test]
    fn session_details_follow_tool_result_visibility() {
        let result = EntityId::from_bytes([3; 16]);
        let model = SessionModel {
            rows: vec![SessionRow {
                reference: result,
                depth: 0,
                label: "Tool result: completed".to_owned(),
                kind: SessionRowKind::ToolResult,
                candidate: None,
                snapshot: None,
                transient: false,
            }],
            execution: scriberig_session::SessionExecutionSelection::default(),
            tool_invocation_details: std::collections::BTreeMap::default(),
        };
        let mut state = TuiState {
            session: LoadState::Ready(model),
            tool_result_details: [(
                result,
                scriberig_protocol::ToolResultDetailResponse {
                    result: result.to_string(),
                    detail: serde_json::json!({"summary": "safe expanded result"}),
                    truncated: false,
                },
            )]
            .into_iter()
            .collect(),
            ..TuiState::default()
        };

        assert!(session_details(&state).contains("Tool result hidden"));
        state.tool_output_visibility = ToolOutputVisibility::Full;
        assert!(session_details(&state).contains("safe expanded result"));
    }
}
