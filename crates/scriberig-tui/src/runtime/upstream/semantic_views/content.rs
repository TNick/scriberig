//! Selects and formats the active `ScribeRig` primary view.

use super::common::{MAX_ROWS, SemanticRole, append_status, append_value};
use super::{model_views, runtime_views};
use crate::selection::PrimaryView;
use crate::state::TuiState;

/// Builds Markdown content for the selected `ScribeRig` semantic view.
///
/// - `state`: Current navigation location and read-model projections.
///
/// # Returns
/// Bounded, terminal-safe content for the upstream Markdown component.
pub(super) fn render_view(state: &TuiState) -> String {
    match state.primary_view {
        PrimaryView::Conversation => conversation(state),
        PrimaryView::Overview => model_views::overview(state),
        PrimaryView::Explorer => model_views::explorer(state),
        PrimaryView::History => model_views::history(state),
        PrimaryView::Find => model_views::find(state),
        PrimaryView::Diff => model_views::diff(state),
        PrimaryView::Projection => model_views::projection(state),
        PrimaryView::Changes => model_views::changes(state),
        PrimaryView::Session => runtime_views::session(state),
        PrimaryView::Operations => runtime_views::operations(state),
        PrimaryView::Plan => model_views::plan(state),
    }
}

/// Renders recursive session and plan rows with their stable ancestry order.
///
/// - `state`: Current conversation and transient operation projections.
///
/// # Returns
/// Bounded conversation content.
fn conversation(state: &TuiState) -> String {
    let mut output = String::from("# Conversation\n\n");
    if let Some(model) = &state.conversation {
        for (index, row) in model.rows.iter().take(MAX_ROWS).enumerate() {
            let kind = format!("{:?}", row.kind);
            let cursor = if state.coordinator_cursor.is_some_and(|(_, node)| {
                row.source == crate::view::conversation::ConversationSource::PlanNode(node)
            }) {
                " · coordinator cursor"
            } else {
                ""
            };
            append_value(
                &mut output,
                &format!(
                    "{}{}[{kind}] {}{cursor}",
                    if index == state.cursor { "› " } else { "  " },
                    "  ".repeat(row.depth.min(8)),
                    row.label
                ),
            );
            if let crate::view::conversation::ConversationSource::Activity(invocation) = row.source
                && state.prompt_visibility == crate::state::PromptVisibility::Shown
                && let Some(detail) = state.model_request_details.get(&invocation)
            {
                append_model_request(&mut output, detail);
            }
        }
        if model.rows.len() > MAX_ROWS {
            append_value(&mut output, "… additional conversation rows omitted");
        }
    }
    if state.reasoning_visibility == crate::state::ReasoningVisibility::Shown {
        for activity in state.transient_agent_activities.iter().take(MAX_ROWS) {
            append_value(
                &mut output,
                &format!(
                    "{} · {:?}: {}",
                    activity.branch, activity.state, activity.summary
                ),
            );
        }
    }
    for response in state.transient_agent_outputs.iter().take(MAX_ROWS) {
        append_value(&mut output, &response.text);
    }
    if let Some(prompt) = state.conversation_composer.pending_prompt.as_deref() {
        append_value(&mut output, &format!("You: {prompt}"));
    }
    if let Some(error) = state.conversation_composer.error.as_deref() {
        append_status(&mut output, "Error", error, SemanticRole::Error);
    }
    if state.conversation.is_none() && state.conversation_composer.pending_prompt.is_none() {
        append_value(&mut output, "Type a message to start a conversation.");
    }
    append_plan_graphs(&mut output, state);
    output
}

/// Renders the exact `ScribeRig` plan DAG separately from the conversation tree.
///
/// - `output`: Conversation content receiving an independent plan section.
/// - `state`: Exact session snapshot containing durable plan definitions.
fn append_plan_graphs(output: &mut String, state: &TuiState) {
    let Some(session) = &state.session_snapshot else {
        return;
    };
    if session.plans.is_empty() {
        return;
    }
    append_value(output, "## Durable plan dependency graphs");
    for plan in session.plans.iter().take(32) {
        append_value(
            output,
            &format!(
                "Plan {} · revision {} · {:?}",
                plan.id, plan.revision, plan.status
            ),
        );
        append_value(output, &format!("Goal: {}", plan.goal));
        if let Some(claim) = &plan.coordinator_claim {
            append_value(output, &format!("Coordinator claim: {}", claim.id));
        }
        if let Some(cursor) = plan.coordinator_cursor {
            append_value(output, &format!("Coordinator cursor: {cursor}"));
        }
        for node in plan.nodes.iter().take(MAX_ROWS) {
            let parent = match node.parent {
                Some(scriberig_session::PlanNodeParent::Branch(branch)) => {
                    format!("session branch {branch}")
                }
                Some(scriberig_session::PlanNodeParent::Node(parent)) => {
                    format!("plan node {parent}")
                }
                None => "plan root".to_owned(),
            };
            append_value(
                output,
                &format!(
                    "{} · {:?} · {} · parent: {parent}",
                    node.title, node.status, node.id
                ),
            );
            if let Some(branch) = node.branch {
                append_value(output, &format!("  execution branch: {branch}"));
            }
            if let Some(result) = node.result {
                append_value(output, &format!("  BranchResult: {result}"));
            }
            if let Some(assignment) = &node.assignment {
                append_value(
                    output,
                    &format!(
                        "  upstream execution: {}/{} · thinking {}",
                        assignment.provider_id,
                        assignment.model_id,
                        assignment.thinking_level.as_deref().unwrap_or("default")
                    ),
                );
            }
        }
        for edge in plan.dependencies.iter().take(MAX_ROWS) {
            append_value(
                output,
                &format!(
                    "Dependency edge: {} → {} · fulfilled by {}",
                    edge.prerequisite,
                    edge.dependent,
                    edge.fulfilled_by
                        .map_or_else(|| "pending".to_owned(), |id| id.to_string())
                ),
            );
        }
    }
}

/// Shows a safe provider-neutral request only when prompt visibility is enabled.
///
/// - `output`: Conversation content receiving the captured request details.
/// - `detail`: Safe request projection associated with a stable invocation ID.
fn append_model_request(
    output: &mut String,
    detail: &scriberig_protocol::ModelRequestDetailResponse,
) {
    append_value(
        output,
        &format!(
            "Effective request · {} · {}",
            detail.invocation, detail.model
        ),
    );
    if let Some(provider) = &detail.provider_id {
        append_value(output, &format!("Provider: {provider}"));
    }
    if let Some(thinking) = &detail.thinking_level {
        append_value(output, &format!("Thinking level: {thinking}"));
    }
    if let Some(profile) = &detail.profile {
        append_value(
            output,
            &format!(
                "Profile revision: {} @ {}",
                profile.identity, profile.revision
            ),
        );
    }
    for component in detail.active_components.iter().take(64) {
        append_value(
            output,
            &format!(
                "Context component: {} @ {}",
                component.identity, component.revision
            ),
        );
    }
    for message in detail.messages.iter().take(64) {
        append_value(output, &format!("{}: {}", message.role, message.content));
    }
    for tool in detail.tools.iter().take(64) {
        append_value(
            output,
            &format!("Tool: {} · {}", tool.name, tool.description),
        );
        if let Some(parameters) = &tool.parameters {
            append_value(output, &format!("Tool parameters: {parameters}"));
        }
    }
    if let Some(report) = &detail.applied_settings {
        let summary = serde_json::to_string(report).unwrap_or_else(|_| "{}".to_owned());
        append_value(output, &format!("Applied settings: {summary}"));
    }
    if detail.truncated {
        append_value(output, "Effective request detail is truncated.");
    }
}
