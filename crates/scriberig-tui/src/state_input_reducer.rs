//! Reduces TUI text-input and submission actions.

#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "state_input_reducer/commands.rs"]
mod commands;
#[path = "state_input_reducer/plan.rs"]
mod plan;

/// Reduces an input-related action and returns its resulting effects.
///
/// - `state`: Mutable TUI state.
/// - `action`: Input, paste, or submission action.
/// - `super`: Parent-module visibility marker.
///
/// Effects produced by the input action.
/// # Returns
/// The operation result.
pub(super) fn reduce_input(state: &mut TuiState, action: Action) -> Vec<Effect> {
    match action {
        Action::InputChanged(text) => {
            if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                input.error = None;
                if text.is_empty() {
                    input.backspace();
                } else {
                    input.append(&text);
                }
            } else if matches!(state.overlay, Some(Overlay::Palette(_))) {
                // The upstream searchable selector owns palette filtering.
            } else if let Some(Overlay::Composer(composer)) = state.overlay.as_mut() {
                edit_composer(&mut composer.text, &mut composer.error, &text);
            } else if state.session_composer_focused
                && let Some(composer) = state.session_composer.as_mut()
            {
                edit_composer(&mut composer.text, &mut composer.error, &text);
            } else if state.primary_view == crate::selection::PrimaryView::Conversation
                && !state.conversation_browsing
            {
                state.conversation_composer.error = None;
                state.conversation_composer.insert_text(&text);
            }
            vec![Effect::None]
        }
        Action::Paste(text) => {
            let text = crate::state::sanitize_paste(&text);
            if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                input.append(&text);
            } else if matches!(state.overlay, Some(Overlay::Palette(_))) {
                // The upstream searchable selector owns palette filtering.
            } else if let Some(Overlay::Composer(composer)) = state.overlay.as_mut() {
                composer.text.push_str(&text);
            } else if state.session_composer_focused
                && let Some(composer) = state.session_composer.as_mut()
            {
                composer.text.push_str(&text);
            } else if state.primary_view == crate::selection::PrimaryView::Conversation
                && !state.conversation_browsing
            {
                state.conversation_composer.error = None;
                state.conversation_composer.paste_text(&text);
            }
            vec![Effect::None]
        }
        Action::InputSubmitted => submit_input(state),
        _ => vec![Effect::None],
    }
}

/// Applies typed text or backspace to either composer implementation.
///
/// - `text`: Current multiline draft.
/// - `error`: Composer validation message to clear after editing.
/// - `input`: Inserted text, or an empty string for backspace.
fn edit_composer(text: &mut String, error: &mut Option<String>, input: &str) {
    *error = None;
    if input.is_empty() {
        text.pop();
    } else {
        text.push_str(input);
    }
}

/// Converts the current overlay text into a semantic effect.
///
/// - `state`: Mutable TUI state containing the active input overlay.
///
/// Effects produced by submitting the active input.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
fn submit_input(state: &mut TuiState) -> Vec<Effect> {
    let text = state.input_text().unwrap_or_default().trim().to_owned();
    if state.primary_view == crate::selection::PrimaryView::Conversation
        && state.overlay.is_none()
        && !state.conversation_browsing
    {
        let mut prompt = state.conversation_composer.draft.clone();
        if prompt.trim().is_empty() {
            return vec![Effect::None];
        }
        if prompt.starts_with("//") {
            if state.conversation_composer.pending {
                return vec![Effect::None];
            }
            prompt.remove(0);
        } else {
            state.conversation_composer.error = None;
            if let Some(effects) = commands::console_command(state, &prompt) {
                if state.conversation_composer.error.is_none() {
                    state.conversation_composer.clear_draft();
                }
                return effects;
            }
            if state.conversation_composer.pending {
                return vec![Effect::None];
            }
        }
        state.conversation_composer.pending_prompt = Some(prompt.clone());
        state.conversation_composer.pending = true;
        state.conversation_composer.error = None;
        state.conversation_composer.clear_draft();
        state.transient_agent_outputs.clear();
        return match state.conversation_composer.target {
            crate::state::ConversationTarget::NewSession => {
                vec![Effect::StartNewConversation {
                    prompt,
                    selection: selected_agent_scope(state),
                }]
            }
            crate::state::ConversationTarget::Existing {
                session,
                reply_anchor,
            } => vec![Effect::ContinueConversation {
                session,
                anchor: reply_anchor,
                prompt,
            }],
        };
    }
    if matches!(state.overlay, Some(Overlay::Composer(_))) {
        let Some(Overlay::Composer(composer)) = state.overlay.take() else {
            return vec![Effect::None];
        };
        if composer.text.trim().is_empty() {
            state.overlay = Some(Overlay::Composer(composer));
            return vec![Effect::None];
        }
        if let Some(effects) = commands::console_command(state, &composer.text) {
            return effects;
        }
        state.session_branch_focus = composer.branch;
        state.transient_agent_outputs.clear();
        return vec![Effect::SubmitAgent {
            session: composer.session,
            branch: composer.branch,
            prompt: composer.text,
        }];
    }
    if state.overlay.is_none() && state.session_composer_focused {
        let command = state
            .session_composer
            .as_ref()
            .map(|composer| composer.text.clone());
        if let Some(command) = command
            && let Some(effects) = commands::console_command(state, &command)
        {
            if let Some(composer) = state.session_composer.as_mut() {
                composer.text.clear();
                composer.pending_prompt = None;
                composer.error = None;
            }
            return effects;
        }
        let (session, branch, prompt) = {
            let Some(composer) = state.session_composer.as_mut() else {
                return vec![Effect::None];
            };
            if composer.pending_prompt.is_some() || composer.text.trim().is_empty() {
                return vec![Effect::None];
            }
            let prompt = std::mem::take(&mut composer.text);
            composer.pending_prompt = Some(prompt.clone());
            composer.error = None;
            (composer.session, composer.branch, prompt)
        };
        state.session_branch_focus = branch;
        state.transient_agent_outputs.clear();
        return vec![Effect::SubmitAgent {
            session,
            branch,
            prompt,
        }];
    }
    if let Some(Overlay::Input(input)) = state.overlay.as_ref()
        && input.purpose == crate::overlay::input::InputPurpose::DiffSnapshot
    {
        let Some(from) = state.selection.and_then(|selection| selection.snapshot) else {
            state.overlay = None;
            "Diff requires an exact base snapshot selection.".clone_into(&mut state.shell_message);
            return vec![Effect::None];
        };
        match text.parse::<scriberig_core::SnapshotId>() {
            Ok(to) if to != from => {
                state.overlay = None;
                return super::reduce(
                    state,
                    Action::DiffRequested {
                        request: scriberig_app::DiffRequest { from, to },
                    },
                );
            }
            Ok(_) => {
                if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                    input.error = Some("target must differ from the base snapshot".to_owned());
                }
            }
            Err(error) => {
                if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                    input.error = Some(format!("invalid snapshot identifier: {error}"));
                }
            }
        }
        return vec![Effect::None];
    }
    if let Some(Overlay::Input(input)) = state.overlay.as_ref()
        && input.purpose == crate::overlay::input::InputPurpose::FindQuery
    {
        if text.is_empty() {
            return vec![Effect::None];
        }
        state.overlay = None;
        return super::reduce(state, Action::FindRequested(text));
    }
    if let Some(Overlay::Input(input)) = state.overlay.as_ref()
        && input.purpose == crate::overlay::input::InputPurpose::ProjectionEdit
    {
        let Some(selection) = state.selection else {
            state.overlay = None;
            return vec![Effect::None];
        };
        let Some(base_snapshot) = selection.snapshot else {
            state.overlay = None;
            return vec![Effect::None];
        };
        if text.is_empty() {
            return vec![Effect::None];
        }
        state.overlay = None;
        return super::reduce(
            state,
            Action::ReviewRequested {
                request: scriberig_app::ApplyRequest {
                    base_snapshot,
                    branch: None,
                    change: format!("project-edit|{}|{}", selection.entity, text),
                },
            },
        );
    }
    if state.is_input_active() && !text.is_empty() {
        match text.parse::<scriberig_core::EntityId>() {
            Ok(entity) => {
                let request_id = state.next_request_id;
                state.next_request_id += 1;
                state.overlay = None;
                return super::reduce(
                    state,
                    Action::InspectRequested {
                        request_id,
                        entity,
                        snapshot: None,
                        switch_view: true,
                    },
                );
            }
            Err(error) => {
                if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                    input.error = Some(format!("invalid entity identifier: {error}"));
                }
            }
        }
    }
    vec![Effect::None]
}

/// Resolves the exact semantic scope selected for a new conversation.
///
/// - `state`: Current TUI semantic scope and navigation selection.
///
/// The project scope with the latest selected entity and exact snapshot.
/// # Returns
/// The agent selection when this TUI has an imported project scope.
fn selected_agent_scope(state: &TuiState) -> Option<scriberig_app::AgentSelection> {
    let scope = state.agent_selection.as_ref()?;
    Some(state.selection.map_or_else(
        || scope.clone(),
        |selection| scriberig_app::AgentSelection {
            entity: selection.entity,
            snapshot: selection.snapshot.unwrap_or(scope.snapshot),
            ..scope.clone()
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::selected_agent_scope;
    use crate::{effect::Effect, selection::EntitySelection, state::TuiState};

    /// Verifies a selected semantic entity enters the next branch context by stable ID.
    #[test]
    fn selected_rich_entity_flows_to_branch_context() {
        let project = scriberig_core::ProjectId::from_bytes([171; 16]);
        let root = scriberig_core::EntityId::from_bytes([172; 16]);
        let selected = scriberig_core::EntityId::from_bytes([173; 16]);
        let snapshot = scriberig_core::SnapshotId::new([174; 32]);
        let state = TuiState {
            agent_selection: Some(scriberig_app::AgentSelection {
                project: Some(project),
                root,
                entity: root,
                snapshot,
            }),
            selection: Some(EntitySelection {
                entity: selected,
                revision: None,
                snapshot: Some(snapshot),
            }),
            ..TuiState::default()
        };

        let scope = selected_agent_scope(&state).expect("selected branch context");

        assert_eq!(scope.entity, selected);
        assert_eq!(scope.root, root);
        assert_eq!(scope.project, Some(project));
        assert_eq!(scope.snapshot, snapshot);
    }

    /// Routes the source-style slash command to the shared preference service.
    #[test]
    fn source_style_command_dispatches_shared_preference_effect() {
        let mut state = TuiState::default();
        let effects = super::commands::console_command(
            &mut state,
            "/source-style indent=tabs,indent-width=2,blank-lines=0",
        )
        .expect("recognized source style command");
        assert!(matches!(
            effects.as_slice(),
            [Effect::SetProjectionStylePreference { request, .. }]
                if request.projection_kind == "rust"
                    && request.scope == scriberig_protocol::ProjectionStyleScope::User
                    && request.style.as_deref() == Some(b"indent=tabs,indent-width=2,blank-lines=0".as_slice())
        ));
    }

    /// Resolves displayed style evidence using the selected project's view identity.
    #[test]
    fn source_style_show_dispatches_exact_effective_style_request() {
        let entity = scriberig_core::EntityId::from_bytes([175; 16]);
        let mut state = TuiState {
            project: Some("warehouse-service".to_owned()),
            selection: Some(EntitySelection {
                entity,
                revision: None,
                snapshot: Some(scriberig_core::SnapshotId::new([176; 32])),
            }),
            ..TuiState::default()
        };
        let effects = super::commands::console_command(&mut state, "/source-style show")
            .expect("recognized source style inspection command");
        assert!(matches!(
            effects.as_slice(),
            [Effect::ResolveProjectionStyle { request, .. }]
                if request.projection_kind == "rust"
                    && request.project_id == "warehouse-service"
                    && request.workspace_id == "warehouse-service"
                    && request.view_id.as_deref() == Some(format!("project-source:{entity}").as_str())
                    && request.purpose == scriberig_protocol::ProjectionStylePurpose::InteractiveView
        ));
    }
}
