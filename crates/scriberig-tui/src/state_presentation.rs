//! Applies terminal-only activity presentation controls.

use crate::{
    effect::Effect,
    inspector::LoadState,
    state::{PresentationControl, ToolOutputVisibility, TuiState},
    view::session::{SessionModel, SessionRowKind},
};

/// Applies shared defaults beneath presentation fields already overridden locally.
///
/// - `state`: Current terminal presentation state.
/// - `response`: Revisioned shared user defaults.
///
/// A no-op effect after updating presentation state.
/// # Returns
/// The operation result.
pub(crate) fn apply_shared_preferences(
    state: &mut TuiState,
    response: scriberig_protocol::AgentPresentationPreferencesResponse,
) -> Vec<Effect> {
    let preferences = response.preferences;
    if !state.has_presentation_override(PresentationControl::ToolOutput) {
        state.tool_output_visibility = match preferences.tool_output {
            scriberig_protocol::PublicToolOutputVisibility::Hidden => ToolOutputVisibility::Hidden,
            scriberig_protocol::PublicToolOutputVisibility::Summary => {
                ToolOutputVisibility::Summary
            }
            scriberig_protocol::PublicToolOutputVisibility::Full => ToolOutputVisibility::Full,
        };
    }
    if !state.has_presentation_override(PresentationControl::ToolInvocation) {
        state.tool_invocation_visibility = match preferences.tool_invocation {
            scriberig_protocol::PublicToolInvocationDetail::Simple => {
                crate::state::ToolInvocationVisibility::CompactTruncated
            }
            scriberig_protocol::PublicToolInvocationDetail::Detailed => {
                crate::state::ToolInvocationVisibility::StructuredParameters
            }
        };
    }
    if !state.has_presentation_override(PresentationControl::Prompt) {
        state.prompt_visibility = match preferences.prompt {
            scriberig_protocol::PublicPromptVisibility::Hidden => {
                crate::state::PromptVisibility::Hidden
            }
            scriberig_protocol::PublicPromptVisibility::Shown => {
                crate::state::PromptVisibility::Shown
            }
        };
    }
    if !state.has_presentation_override(PresentationControl::Reasoning) {
        state.reasoning_visibility = match preferences.reasoning {
            scriberig_protocol::PublicReasoningVisibility::Hidden => {
                crate::state::ReasoningVisibility::Hidden
            }
            scriberig_protocol::PublicReasoningVisibility::Shown => {
                crate::state::ReasoningVisibility::Shown
            }
        };
    }
    state.presentation_preferences_revision = Some(response.revision);
    vec![Effect::None]
}

/// Toggles a supported presentation-only terminal control.
///
/// - `state`: TUI presentation state to update.
/// - `item_id`: Stable palette action identity.
///
/// Effects for recognized controls, including safe detail reads when needed.
/// # Returns
/// The operation result.
pub(crate) fn activate_presentation_action(
    state: &mut TuiState,
    item_id: &str,
) -> Option<Vec<Effect>> {
    if item_id == "toggle-reasoning" {
        state.set_presentation_override(PresentationControl::Reasoning);
        state.reasoning_visibility = match state.reasoning_visibility {
            crate::state::ReasoningVisibility::Hidden => crate::state::ReasoningVisibility::Shown,
            crate::state::ReasoningVisibility::Shown => crate::state::ReasoningVisibility::Hidden,
        };
        state.shell_message = match state.reasoning_visibility {
            crate::state::ReasoningVisibility::Hidden => {
                "Safe reasoning summaries hidden for this TUI session.".to_owned()
            }
            crate::state::ReasoningVisibility::Shown => {
                "Safe reasoning summaries shown for this TUI session.".to_owned()
            }
        };
        return Some(vec![Effect::None]);
    }
    if item_id == "toggle-prompt" {
        state.set_presentation_override(PresentationControl::Prompt);
        state.prompt_visibility = match state.prompt_visibility {
            crate::state::PromptVisibility::Hidden => crate::state::PromptVisibility::Shown,
            crate::state::PromptVisibility::Shown => crate::state::PromptVisibility::Hidden,
        };
        return Some(
            if state.prompt_visibility == crate::state::PromptVisibility::Shown {
                read_missing_model_requests(state)
            } else {
                "Captured model requests hidden for this TUI session."
                    .clone_into(&mut state.shell_message);
                vec![Effect::None]
            },
        );
    }
    if item_id != "toggle-tool-output" {
        if item_id == "toggle-tool-invocation" {
            state.set_presentation_override(PresentationControl::ToolInvocation);
            state.tool_invocation_visibility = match state.tool_invocation_visibility {
                crate::state::ToolInvocationVisibility::Hidden => {
                    crate::state::ToolInvocationVisibility::CompactTruncated
                }
                crate::state::ToolInvocationVisibility::CompactTruncated => {
                    crate::state::ToolInvocationVisibility::Full
                }
                crate::state::ToolInvocationVisibility::Full => {
                    crate::state::ToolInvocationVisibility::StructuredParameters
                }
                crate::state::ToolInvocationVisibility::StructuredParameters => {
                    crate::state::ToolInvocationVisibility::Hidden
                }
            };
            state.shell_message = match state.tool_invocation_visibility {
                crate::state::ToolInvocationVisibility::Hidden => {
                    "Tool-call arguments hidden for this TUI session.".to_owned()
                }
                crate::state::ToolInvocationVisibility::CompactTruncated => {
                    "Compact tool-call display enabled for this TUI session.".to_owned()
                }
                crate::state::ToolInvocationVisibility::Full => {
                    "Full bounded safe tool-call display enabled for this TUI session.".to_owned()
                }
                crate::state::ToolInvocationVisibility::StructuredParameters => {
                    "Structured safe tool-call parameters enabled for this TUI session.".to_owned()
                }
            };
            return Some(vec![Effect::None]);
        }
        return None;
    }
    match state.tool_output_visibility {
        ToolOutputVisibility::Hidden => {
            state.set_presentation_override(PresentationControl::ToolOutput);
            state.tool_output_visibility = ToolOutputVisibility::Summary;
            "Tool result summaries shown for this TUI session."
                .clone_into(&mut state.shell_message);
            Some(vec![Effect::None])
        }
        ToolOutputVisibility::Summary => {
            state.set_presentation_override(PresentationControl::ToolOutput);
            state.tool_output_visibility = ToolOutputVisibility::Full;
            "Loading bounded tool-result details…".clone_into(&mut state.shell_message);
            Some(read_missing_tool_details(state))
        }
        ToolOutputVisibility::Full => {
            state.set_presentation_override(PresentationControl::ToolOutput);
            state.tool_output_visibility = ToolOutputVisibility::Hidden;
            "Tool result summaries hidden for this TUI session."
                .clone_into(&mut state.shell_message);
            Some(vec![Effect::None])
        }
    }
}

/// Requests captured model requests for visible invocations without cached detail.
///
/// - `state`: TUI state holding the loaded session activity.
///
/// One safe detail-read effect per uncached invocation, or a no-op.
/// # Returns
/// The operation result.
fn read_missing_model_requests(state: &mut TuiState) -> Vec<Effect> {
    let LoadState::Ready(model) = &state.session else {
        return vec![Effect::None];
    };
    let Some(session) = model
        .rows
        .iter()
        .find(|row| row.kind == SessionRowKind::Session)
        .map(|row| row.reference)
    else {
        return vec![Effect::None];
    };
    let pending = invocation_branches(model)
        .into_iter()
        .filter(|(invocation, _)| !state.model_request_details.contains_key(invocation))
        .collect::<Vec<_>>();
    if pending.is_empty() {
        "Captured model requests shown for this TUI session.".clone_into(&mut state.shell_message);
        return vec![Effect::None];
    }
    "Loading captured model requests…".clone_into(&mut state.shell_message);
    pending
        .into_iter()
        .map(|(invocation, branch)| Effect::ReadModelRequestDetail {
            session,
            branch,
            invocation,
        })
        .collect()
}

/// Resolves each model-invocation row to its nearest enclosing branch.
///
/// - `model`: Loaded TUI session projection.
///
/// Stable invocation and branch identity pairs.
/// # Returns
/// The operation result.
fn invocation_branches(
    model: &SessionModel,
) -> Vec<(scriberig_core::EntityId, scriberig_core::EntityId)> {
    model
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.kind == SessionRowKind::ModelInvocation)
        .filter_map(|(index, row)| {
            let branch = model.rows[..index].iter().rev().find(|candidate| {
                candidate.kind == SessionRowKind::Branch && candidate.depth < row.depth
            })?;
            Some((row.reference, branch.reference))
        })
        .collect()
}

/// Requests detail for visible results that are not cached yet.
///
/// - `state`: TUI state holding the loaded session activity.
///
/// One detail-read effect per uncached tool result, or a no-op.
/// # Returns
/// The operation result.
fn read_missing_tool_details(state: &TuiState) -> Vec<Effect> {
    let LoadState::Ready(model) = &state.session else {
        return vec![Effect::None];
    };
    let Some(session) = model
        .rows
        .iter()
        .find(|row| row.kind == SessionRowKind::Session)
        .map(|row| row.reference)
    else {
        return vec![Effect::None];
    };
    let mut effects = result_branches(model)
        .into_iter()
        .filter(|(result, _)| !state.tool_result_details.contains_key(result))
        .map(|(result, branch)| Effect::ReadToolResultDetail {
            session,
            branch,
            result,
        })
        .collect::<Vec<_>>();
    if effects.is_empty() {
        effects.push(Effect::None);
    }
    effects
}

/// Resolves each tool-result row to its nearest preceding branch row.
///
/// - `model`: Loaded TUI session projection.
///
/// Stable result and branch identity pairs.
/// # Returns
/// The operation result.
fn result_branches(
    model: &SessionModel,
) -> Vec<(scriberig_core::EntityId, scriberig_core::EntityId)> {
    model
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.kind == SessionRowKind::ToolResult)
        .filter_map(|(index, row)| {
            let branch = model.rows[..index].iter().rev().find(|candidate| {
                candidate.kind == SessionRowKind::Branch && candidate.depth < row.depth
            })?;
            Some((row.reference, branch.reference))
        })
        .collect()
}

/// Routes any presentation or model-selection palette action.
///
/// - `state`: TUI state to update.
/// - `item_id`: Stable palette action identity.
///
/// Effects when the action belongs to an agent-console control.
/// # Returns
/// The operation result.
pub(crate) fn activate_palette_control(state: &mut TuiState, item_id: &str) -> Option<Vec<Effect>> {
    if let Some(model_id) = item_id.strip_prefix("assign-plan-model:") {
        let target = state.plan_assignment_target.take()?;
        let LoadState::Ready(library) = &state.model_library else {
            state.conversation_composer.error =
                Some("Reload models before assigning this plan node.".to_owned());
            state.overlay = None;
            return Some(vec![Effect::None]);
        };
        let Some(model) = library.models.iter().find(|model| model.id == model_id) else {
            state.conversation_composer.error =
                Some("The selected model is no longer available.".to_owned());
            state.overlay = None;
            return Some(vec![Effect::None]);
        };
        let Some(session) = state.session_snapshot.as_ref().map(|session| session.id) else {
            state.conversation_composer.error =
                Some("Load the plan session before assigning a node.".to_owned());
            state.overlay = None;
            return Some(vec![Effect::None]);
        };
        state.overlay = None;
        return Some(vec![Effect::SetPlanNodeAssignment(
            scriberig_app::PlanAssignmentRequest {
                session,
                plan: target.plan,
                node: target.node,
                expected_revision: target.expected_revision,
                assignment: Some(scriberig_session::ModelAssignment {
                    provider_id: model.provider_id.clone(),
                    model_id: model.provider_model_id.clone(),
                    thinking_level: model.thinking_levels.first().cloned(),
                }),
            },
        )]);
    }
    activate_presentation_action(state, item_id)
}

#[cfg(test)]
#[path = "state_presentation_tests.rs"]
mod tests;
