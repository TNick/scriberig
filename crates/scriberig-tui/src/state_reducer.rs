//! Reduces TUI actions into state and effects.

#![allow(clippy::wildcard_imports)]
use super::state_actions::activate_current;
use super::state_cursor_reducer::reduce_cursor;
use super::state_cursor_reducer::reduce_focus;
use super::state_reducer_tail::reduce_tail;
use super::*;
use crate::{overlay::palette::PaletteState, state::ConnectionState};
/// Applies one action deterministically and returns work for the runtime.
/// - `state`: Argument named `state`.
/// - `action`: Argument named `action`.
/// # Returns
/// The value produced by the declaration.
#[allow(clippy::too_many_lines)]
pub fn reduce(state: &mut TuiState, action: Action) -> Vec<Effect> {
    if matches!(
        action,
        Action::SessionSucceeded { .. }
            | Action::AgentSubmitted(_)
            | Action::SessionRequested(_)
            | Action::ComposerOpened { .. }
            | Action::AgentOutputObserved(_)
    ) {
        return reduce_tail(state, action).unwrap_or_default();
    }
    match action {
        Action::Noop => vec![Effect::None],
        Action::QuitRequested => {
            state.should_quit = true;
            vec![Effect::RequestQuit]
        }
        Action::OverlayClosed | Action::CancelReview => {
            state.overlay = None;
            vec![Effect::None]
        }
        Action::FocusNext => reduce_focus(state, false),
        Action::FocusPrevious => reduce_focus(state, true),
        Action::CursorMoved(direction) => reduce_cursor(state, direction),
        Action::Activated => activate_current(state),
        Action::ActivateLocation(location) => {
            state.navigation.activate(location);
            vec![Effect::None]
        }
        Action::Back => {
            state.navigation.go_back();
            vec![Effect::None]
        }
        Action::Forward => {
            state.navigation.go_forward();
            vec![Effect::None]
        }
        Action::RefreshRequested => {
            let Some(selection) = state.selection else {
                return vec![Effect::None];
            };
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.inspector = LoadState::Loading { request_id };
            vec![Effect::InspectEntity {
                request_id,
                entity: selection.entity,
                snapshot: selection.snapshot,
            }]
        }
        Action::HelpOpened => {
            state.overlay = Some(Overlay::Help);
            vec![Effect::None]
        }
        Action::PaletteOpened => {
            state.overlay = Some(Overlay::Palette(PaletteState::with_capabilities(
                &state.capabilities,
            )));
            vec![Effect::None]
        }
        Action::FindOpened => {
            state.overlay = Some(Overlay::Input(InputState::default()));
            vec![Effect::None]
        }
        Action::InputChanged(text) => {
            if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                input.error = None;
                if text.is_empty() {
                    input.backspace();
                } else {
                    input.append(&text);
                }
            } else if let Some(Overlay::Palette(palette)) = state.overlay.as_mut() {
                if text.is_empty() {
                    palette.query.pop();
                    palette.selected = palette
                        .selected
                        .min(palette.filtered().len().saturating_sub(1));
                } else {
                    palette.set_query(format!("{}{}", palette.query, text));
                }
            } else if let Some(Overlay::Composer(composer)) = state.overlay.as_mut() {
                composer.error = None;
                if text.is_empty() {
                    composer.text.pop();
                } else {
                    composer.text.push_str(&text);
                }
            }
            vec![Effect::None]
        }
        Action::Paste(text) => {
            if let Some(Overlay::Input(input)) = state.overlay.as_mut() {
                input.append(&text);
            } else if let Some(Overlay::Palette(palette)) = state.overlay.as_mut() {
                palette.set_query(format!("{}{}", palette.query, text));
            } else if let Some(Overlay::Composer(composer)) = state.overlay.as_mut() {
                composer.text.push_str(&text);
            }
            vec![Effect::None]
        }
        Action::InputSubmitted => {
            let text = state.input_text().unwrap_or_default().trim().to_owned();
            if matches!(state.overlay, Some(Overlay::Composer(_))) {
                let Some(Overlay::Composer(composer)) = state.overlay.take() else {
                    return vec![Effect::None];
                };
                if composer.text.trim().is_empty() {
                    state.overlay = Some(Overlay::Composer(composer));
                    return vec![Effect::None];
                }
                return vec![Effect::SubmitAgent {
                    session: composer.session,
                    branch: composer.branch,
                    prompt: composer.text,
                }];
            }
            if state.is_input_active() && !text.is_empty() {
                match text.parse::<scriberig_core::EntityId>() {
                    Ok(entity) => {
                        let request_id = state.next_request_id;
                        state.next_request_id += 1;
                        state.overlay = None;
                        return reduce(
                            state,
                            Action::InspectRequested {
                                request_id,
                                entity,
                                snapshot: None,
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
        Action::TerminalResized { width, height } => {
            state.terminal_size = (width, height);
            vec![Effect::None]
        }
        Action::InspectRequested {
            request_id,
            entity,
            snapshot,
        } => {
            state.selection = Some(EntitySelection {
                entity,
                revision: None,
                snapshot,
            });
            state.explorer = ExplorerModel::from_selection(EntitySelection {
                entity,
                revision: None,
                snapshot,
            });
            state.inspector = LoadState::Loading { request_id };
            vec![Effect::InspectEntity {
                request_id,
                entity,
                snapshot,
            }]
        }
        Action::InspectSucceeded { request_id, model } => {
            state.connection = ConnectionState::Connected;
            state.stale = false;
            if matches!(state.inspector, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.selection = Some(EntitySelection {
                    entity: model.entity,
                    revision: model.revision,
                    snapshot: model.snapshot,
                });
                state.inspector = LoadState::Ready(model.clone());
                state
                    .navigation
                    .activate(crate::selection::NavigationLocation {
                        selection: Some(crate::selection::SemanticReference(
                            model.entity.to_string(),
                        )),
                        entity: state.selection,
                        ..Default::default()
                    });
            }
            vec![Effect::None]
        }
        Action::InspectFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.inspector, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.inspector = LoadState::Failed {
                    category,
                    message,
                    stale: false,
                };
            }
            vec![Effect::None]
        }
        Action::CapabilitiesLoaded(capabilities) => {
            state.capabilities = capabilities;
            vec![Effect::None]
        }
        Action::ViewSelected(view) => {
            state.primary_view = view;
            vec![Effect::None]
        }
        Action::HistorySucceeded { request_id, model } => {
            if matches!(state.history, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.history = LoadState::Ready(model);
            }
            vec![Effect::None]
        }
        Action::DiffSucceeded { request_id, model } => {
            if matches!(state.diff, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.diff = LoadState::Ready(model);
            }
            vec![Effect::None]
        }
        Action::ProjectionSucceeded { request_id, model } => {
            if matches!(state.projection, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.projection = LoadState::Ready(model);
            }
            vec![Effect::None]
        }
        Action::ReviewSucceeded { request_id, model } => {
            if matches!(state.review, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.review = LoadState::Ready(model);
            }
            vec![Effect::None]
        }
        Action::OperationFailed {
            request_id,
            category,
            message,
        } => {
            if category == "transport_unavailable" {
                state.connection = ConnectionState::Disconnected;
                state.stale = true;
            } else {
                state.connection = ConnectionState::Failed;
            }
            state.operation_error = Some((category.clone(), message.clone()));
            if matches!(state.history, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.history = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            if matches!(state.diff, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.diff = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            if matches!(state.projection, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.projection = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            if matches!(state.review, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.review = LoadState::Failed {
                    category,
                    message,
                    stale: state.review_proposal.is_some(),
                };
            }
            vec![Effect::None]
        }
        Action::ConfirmReview => {
            if let Some(Overlay::Confirm(confirm)) = state.overlay.take() {
                return reduce(state, Action::CommitConfirmed(confirm.commit));
            }
            if let LoadState::Ready(model) = &state.review {
                let commit = crate::view::changes::ConfirmedCommit {
                    base_snapshot: model.base_snapshot,
                    change: model.change.clone(),
                    branch: model.branch.clone(),
                };
                state.overlay = Some(Overlay::Confirm(crate::overlay::ConfirmationState {
                    action: "commit change".to_owned(),
                    target: model.change.clone(),
                    base: model.base_snapshot.to_string(),
                    summary: format!(
                        "{} affected semantic objects",
                        model.affected_entities.len()
                    ),
                    commit,
                }));
            }
            vec![Effect::None]
        }
        Action::CommitConfirmed(commit) => {
            state.overlay = None;
            if let LoadState::Ready(model) = &state.review {
                state.review_proposal = Some(model.clone());
            }
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.review = LoadState::Loading { request_id };
            vec![Effect::Commit {
                request_id,
                request: scriberig_app::CommitRequest {
                    base_snapshot: commit.base_snapshot,
                    branch: commit.branch,
                    change: commit.change,
                },
            }]
        }
        Action::HistoryRequested { snapshot } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::History;
            state.history = LoadState::Loading { request_id };
            vec![Effect::History {
                request_id,
                snapshot,
            }]
        }
        Action::DiffRequested { request } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::Diff;
            state.diff = LoadState::Loading { request_id };
            vec![Effect::Diff {
                request_id,
                request,
            }]
        }
        Action::ProjectionRequested { request } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::Projection;
            state.projection = LoadState::Loading { request_id };
            vec![Effect::Render {
                request_id,
                request,
            }]
        }
        Action::ReviewRequested { request } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::Changes;
            state.review = LoadState::Loading { request_id };
            vec![Effect::Apply {
                request_id,
                request,
            }]
        }
        Action::OperationObserved(model) => {
            state.pending_operation_refresh.retain(|id| id != &model.id);
            if let Some(existing) = state.operations.iter_mut().find(|item| item.id == model.id) {
                *existing = model;
            } else {
                state.operations.push(model);
            }
            vec![Effect::None]
        }
        Action::OperationStatusRequested(operation) => {
            vec![Effect::OperationStatus { operation }]
        }
        Action::OperationCancelRequested(operation) => {
            vec![Effect::CancelOperation { operation }]
        }
        Action::ConnectionRestored => {
            state.connection = ConnectionState::Connected;
            state.stale = true;
            if state.selection.is_some() {
                reduce(state, Action::RefreshRequested)
            } else {
                vec![Effect::None]
            }
        }
        _ => state_reducer_tail::reduce_tail(state, action).unwrap_or_default(),
    }
}
