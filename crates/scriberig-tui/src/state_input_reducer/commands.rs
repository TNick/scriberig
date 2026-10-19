//! Routes recognized conversation slash commands to local effects.

use super::*;

/// Routes supported console commands to shared picker and preference actions.
///
/// - `state`: Current TUI state used by the shared action reducers.
/// - `text`: Submitted composer text.
///
/// Effects for a recognized command, or `None` for an agent prompt.
/// # Returns
/// The operation result.
#[allow(clippy::too_many_lines)] // Slash routing remains one descriptor-driven command reducer.
pub(super) fn console_command(state: &mut TuiState, text: &str) -> Option<Vec<Effect>> {
    let command = text.trim();
    let name = command.split_whitespace().next().unwrap_or(command);
    let Some(descriptor) = crate::slash_commands::find(name) else {
        if command.starts_with('/') {
            state.conversation_composer.error =
                Some(format!("Unknown command: {name}. Use /help for commands."));
            return Some(vec![Effect::None]);
        }
        return None;
    };
    match command {
        "/model" => {
            super::super::state_model_picker::activate_model_picker_action(state, "model-picker")
        }
        "/thinking" => {
            super::super::state_model_picker::activate_model_picker_action(state, "thinking-picker")
        }
        "/view reasoning" => super::super::state_presentation::activate_presentation_action(
            state,
            "toggle-reasoning",
        ),
        "/view prompt" => {
            super::super::state_presentation::activate_presentation_action(state, "toggle-prompt")
        }
        "/view tool-call" => super::super::state_presentation::activate_presentation_action(
            state,
            "toggle-tool-invocation",
        ),
        "/view tool-output" => super::super::state_presentation::activate_presentation_action(
            state,
            "toggle-tool-output",
        ),
        _ if descriptor.name == "/help" => {
            state.overlay = Some(crate::overlay::Overlay::Help);
            Some(vec![Effect::None])
        }
        _ if descriptor.name == "/new" => {
            state.conversation_composer.target = crate::state::ConversationTarget::NewSession;
            Some(vec![Effect::None])
        }
        _ if descriptor.name == "/edit" => {
            Some(super::super::state_actions::begin_projection_edit(state))
        }
        _ if descriptor.name == "/session" => {
            if matches!(state.session, crate::inspector::LoadState::Ready(_)) {
                state.primary_view = crate::selection::PrimaryView::Session;
                Some(vec![Effect::None])
            } else {
                state.conversation_composer.error = Some("No session is loaded yet.".to_owned());
                Some(vec![Effect::None])
            }
        }
        _ if descriptor.name == "/explorer" => {
            let Some(selection) = state.selection else {
                state.conversation_composer.error =
                    Some("Select a semantic entity before opening Explorer.".to_owned());
                return Some(vec![Effect::None]);
            };
            let Some(snapshot) = selection.snapshot else {
                state.conversation_composer.error =
                    Some("Explorer requires an exact snapshot selection.".to_owned());
                return Some(vec![Effect::None]);
            };
            state.primary_view = crate::selection::PrimaryView::Explorer;
            state.focus = crate::state::Focus::Explorer;
            state.cursor = 0;
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            Some(vec![Effect::ListExplorer {
                request_id,
                entity: selection.entity,
                snapshot,
                continuation: None,
            }])
        }
        _ if descriptor.name == "/plan" => {
            let argument = command.strip_prefix("/plan").unwrap_or_default().trim();
            if argument.is_empty() {
                state.conversation_composer.error = Some(
                    "Use /plan <goal>, /plan show, or /plan start|pause|resume|stop|complete."
                        .to_owned(),
                );
                return Some(vec![Effect::None]);
            }
            if argument == "show" {
                let Some(session) = state.session_snapshot.as_ref() else {
                    state.conversation_composer.error =
                        Some("Load a session before showing a plan.".to_owned());
                    return Some(vec![Effect::None]);
                };
                let selected_plan = match state.conversation_cursor {
                    Some(crate::view::conversation::ConversationSource::Plan(plan)) => Some(plan),
                    Some(crate::view::conversation::ConversationSource::PlanNode(node)) => session
                        .plans
                        .iter()
                        .find(|plan| plan.nodes.iter().any(|item| item.id == node))
                        .map(|plan| plan.id),
                    _ => None,
                };
                let plan = selected_plan
                    .and_then(|id| session.plans.iter().find(|plan| plan.id == id))
                    .or_else(|| session.plans.last());
                let Some(plan) = plan else {
                    state.conversation_composer.error =
                        Some("This session has no plan yet.".to_owned());
                    return Some(vec![Effect::None]);
                };
                let completed = plan
                    .nodes
                    .iter()
                    .filter(|node| node.result.is_some())
                    .count();
                state.shell_message = format!(
                    "Plan: {} · {:?} · revision {} · {}/{} results · {} dependencies",
                    plan.goal,
                    plan.status,
                    plan.revision,
                    completed,
                    plan.nodes.len(),
                    plan.dependencies.len()
                );
                return Some(vec![Effect::None]);
            }
            if let Some(reason) = argument.strip_prefix("replan ").map(str::trim)
                && !reason.is_empty()
            {
                let Some(session) = state.session_snapshot.as_ref() else {
                    state.conversation_composer.error =
                        Some("Load a session before revising a plan.".to_owned());
                    return Some(vec![Effect::None]);
                };
                let selected_plan = match state.conversation_cursor {
                    Some(crate::view::conversation::ConversationSource::Plan(plan)) => Some(plan),
                    Some(crate::view::conversation::ConversationSource::PlanNode(node)) => session
                        .plans
                        .iter()
                        .find(|plan| plan.nodes.iter().any(|item| item.id == node))
                        .map(|plan| plan.id),
                    _ => None,
                };
                let plan = selected_plan
                    .and_then(|id| session.plans.iter().find(|plan| plan.id == id))
                    .or_else(|| session.plans.last());
                let Some(plan) = plan else {
                    state.conversation_composer.error =
                        Some("Create a plan before revising it.".to_owned());
                    return Some(vec![Effect::None]);
                };
                let title = reason.lines().next().unwrap_or("Replanned work");
                let title = title.chars().take(80).collect::<String>();
                return Some(vec![Effect::Replan(scriberig_app::PlanReplanRequest {
                    session: session.id,
                    plan: plan.id,
                    expected_revision: plan.revision,
                    additions: vec![scriberig_app::PlanNodeDraft {
                        key: "replan-addition".to_owned(),
                        parent_index: None,
                        title,
                        purpose: reason.to_owned(),
                        expected_result: Some(scriberig_session::ExpectedResult {
                            kind: "summary".to_owned(),
                            requires_handle: false,
                        }),
                        semantic_scope: Vec::new(),
                        assignment: None,
                    }],
                    dependencies: Vec::new(),
                    removed_dependencies: Vec::new(),
                    superseded: Vec::new(),
                    assignment_changes: Vec::new(),
                    preserved_terminal_nodes: plan
                        .nodes
                        .iter()
                        .filter(|node| node.result.is_some())
                        .map(|node| node.id)
                        .collect(),
                    reason: reason.to_owned(),
                })]);
            }
            if let Some(action) = match argument {
                "start" => Some(scriberig_app::PlanLifecycleAction::Start),
                "pause" => Some(scriberig_app::PlanLifecycleAction::Pause),
                "resume" => Some(scriberig_app::PlanLifecycleAction::Resume),
                "stop" => Some(scriberig_app::PlanLifecycleAction::Stop),
                "complete" => Some(scriberig_app::PlanLifecycleAction::Complete),
                _ => None,
            } {
                let Some(session) = state.session_snapshot.as_ref() else {
                    state.conversation_composer.error =
                        Some("Load a session before controlling a plan.".to_owned());
                    return Some(vec![Effect::None]);
                };
                let selected_plan = match state.conversation_cursor {
                    Some(crate::view::conversation::ConversationSource::Plan(plan)) => Some(plan),
                    Some(crate::view::conversation::ConversationSource::PlanNode(node)) => session
                        .plans
                        .iter()
                        .find(|plan| plan.nodes.iter().any(|item| item.id == node))
                        .map(|plan| plan.id),
                    _ => None,
                };
                let plan = selected_plan
                    .and_then(|id| session.plans.iter().find(|plan| plan.id == id))
                    .or_else(|| session.plans.last());
                let Some(plan) = plan else {
                    state.conversation_composer.error =
                        Some("Create a plan before changing its lifecycle.".to_owned());
                    return Some(vec![Effect::None]);
                };
                return Some(vec![Effect::PlanLifecycle {
                    request: scriberig_app::PlanLifecycleRequest {
                        session: session.id,
                        plan: plan.id,
                        expected_revision: plan.revision,
                    },
                    action,
                }]);
            }
            let crate::state::ConversationTarget::Existing {
                session,
                reply_anchor,
            } = state.conversation_composer.target
            else {
                state.conversation_composer.error =
                    Some("Start a conversation before creating a plan.".to_owned());
                return Some(vec![Effect::None]);
            };
            let draft = match plan::parse_plan_argument(argument) {
                Ok(draft) => draft,
                Err(message) => {
                    state.conversation_composer.error = Some(message);
                    return Some(vec![Effect::None]);
                }
            };
            Some(vec![Effect::CreatePlan(scriberig_app::PlanCreateRequest {
                session,
                anchor: reply_anchor,
                goal: draft.goal,
                nodes: draft.nodes,
                dependencies: draft.dependencies,
            })])
        }
        _ if descriptor.name == "/assign" => {
            let Some(crate::view::conversation::ConversationSource::PlanNode(node)) =
                state.conversation_cursor
            else {
                state.conversation_composer.error =
                    Some("Select a plan node before using /assign.".to_owned());
                return Some(vec![Effect::None]);
            };
            let Some(plan) = state.session_snapshot.as_ref().and_then(|session| {
                session
                    .plans
                    .iter()
                    .find(|plan| plan.nodes.iter().any(|candidate| candidate.id == node))
            }) else {
                state.conversation_composer.error =
                    Some("The selected plan node is no longer available.".to_owned());
                return Some(vec![Effect::None]);
            };
            state.plan_assignment_target = Some(crate::state::PlanAssignmentTarget {
                plan: plan.id,
                node,
                expected_revision: plan.revision,
            });
            if let crate::inspector::LoadState::Ready(library) = &state.model_library {
                let library = library.clone();
                super::super::state_model_picker::show_plan_assignment_picker(state, &library);
                return Some(vec![Effect::None]);
            }
            if matches!(
                state.model_library,
                crate::inspector::LoadState::Loading { .. }
            ) {
                return Some(vec![Effect::None]);
            }
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.model_library = crate::inspector::LoadState::Loading { request_id };
            Some(vec![Effect::ReadModelLibrary { request_id }])
        }
        _ if descriptor.name == "/operations" => Some(super::reduce(
            state,
            Action::ViewSelected(crate::selection::PrimaryView::Operations),
        )),
        _ if descriptor.name == "/project" => {
            state.shell_message = state.project.as_ref().map_or_else(
                || "No active project is selected.".to_owned(),
                |name| format!("Active project: {name}"),
            );
            Some(vec![Effect::None])
        }
        _ if descriptor.name == "/history" => {
            let Some(snapshot) = state.selection.and_then(|selection| selection.snapshot) else {
                state.conversation_composer.error =
                    Some("Select an exact snapshot before opening history.".to_owned());
                return Some(vec![Effect::None]);
            };
            Some(super::reduce(state, Action::HistoryRequested { snapshot }))
        }
        _ if descriptor.name == "/diff" => Some(super::reduce(state, Action::DiffChooserOpened)),
        _ if descriptor.name == "/projection" => {
            Some(super::reduce(state, Action::ProjectionOpened))
        }
        _ if descriptor.name == "/changes" => {
            Some(super::super::state_actions::refresh_candidate_review(state))
        }
        _ if descriptor.name == "/find" && command.len() > name.len() => Some(super::reduce(
            state,
            Action::FindRequested(command[name.len()..].trim().to_owned()),
        )),
        _ if descriptor.name == "/inspect" => {
            let entity = command
                .split_whitespace()
                .nth(1)
                .and_then(|value| value.parse().ok())
                .or_else(|| state.selection.map(|selection| selection.entity));
            let Some(entity) = entity else {
                state.conversation_composer.error =
                    Some("Select an entity or provide its identifier.".to_owned());
                return Some(vec![Effect::None]);
            };
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            Some(super::reduce(
                state,
                Action::InspectRequested {
                    request_id,
                    entity,
                    snapshot: None,
                },
            ))
        }
        _ if descriptor.name == "/status" => {
            state.conversation_composer.error = Some(format!(
                "Connection: {:?} · session: {}",
                state.connection,
                state
                    .session_snapshot
                    .as_ref()
                    .map_or("none".to_owned(), |session| session.id.to_string())
            ));
            Some(vec![Effect::None])
        }
        _ => {
            state.conversation_composer.error = Some(format!(
                "{} is recognized but requires an active shared view or plan target.",
                descriptor.name
            ));
            Some(vec![Effect::None])
        }
    }
}
