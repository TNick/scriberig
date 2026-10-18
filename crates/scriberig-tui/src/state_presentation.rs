//! Applies terminal-only activity presentation controls.

use crate::{
    effect::Effect,
    inspector::LoadState,
    state::{PresentationControl, ToolOutputVisibility, TuiState},
    view::session::{SessionModel, SessionRowKind},
};

/// Applies shared defaults beneath presentation fields already overridden locally.
///
/// # Parameters
/// - `state`: Current terminal presentation state.
/// - `response`: Revisioned shared user defaults.
///
/// # Returns
/// A no-op effect after updating presentation state.
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
                crate::state::ToolInvocationVisibility::Simple
            }
            scriberig_protocol::PublicToolInvocationDetail::Detailed => {
                crate::state::ToolInvocationVisibility::Detailed
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
/// # Parameters
/// - `state`: TUI presentation state to update.
/// - `item_id`: Stable palette action identity.
///
/// # Returns
/// Effects for recognized controls, including safe detail reads when needed.
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
                crate::state::ToolInvocationVisibility::Simple => {
                    crate::state::ToolInvocationVisibility::Detailed
                }
                crate::state::ToolInvocationVisibility::Detailed => {
                    crate::state::ToolInvocationVisibility::Simple
                }
            };
            state.shell_message = match state.tool_invocation_visibility {
                crate::state::ToolInvocationVisibility::Simple => {
                    "Simple tool-call display enabled for this TUI session.".to_owned()
                }
                crate::state::ToolInvocationVisibility::Detailed => {
                    "Detailed safe tool-call display enabled for this TUI session.".to_owned()
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
/// # Parameters
/// - `state`: TUI state holding the loaded session activity.
///
/// # Returns
/// One safe detail-read effect per uncached invocation, or a no-op.
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
/// # Parameters
/// - `model`: Loaded TUI session projection.
///
/// # Returns
/// Stable invocation and branch identity pairs.
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
/// # Parameters
/// - `state`: TUI state holding the loaded session activity.
///
/// # Returns
/// One detail-read effect per uncached tool result, or a no-op.
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
/// # Parameters
/// - `model`: Loaded TUI session projection.
///
/// # Returns
/// Stable result and branch identity pairs.
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
/// # Parameters
/// - `state`: TUI state to update.
/// - `item_id`: Stable palette action identity.
///
/// # Returns
/// Effects when the action belongs to an agent-console control.
pub(crate) fn activate_palette_control(state: &mut TuiState, item_id: &str) -> Option<Vec<Effect>> {
    super::state_model_picker::activate_model_picker_action(state, item_id)
        .or_else(|| activate_presentation_action(state, item_id))
}

#[cfg(test)]
#[path = "state_presentation_tests.rs"]
mod tests;
