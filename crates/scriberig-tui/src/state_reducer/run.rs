//! Applies one TUI action to presentation state.

use super::*;

#[path = "run/connection_restored.rs"]
mod connection_restored;

/// Applies one action deterministically and returns work for the runtime.
/// - `state`: Argument named `state`.
/// - `action`: Argument named `action`.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
pub fn reduce(state: &mut TuiState, action: Action) -> Vec<Effect> {
    if !matches!(action, Action::CursorMoved(_))
        || !matches!(
            state.primary_view,
            crate::selection::PrimaryView::Conversation | crate::selection::PrimaryView::Session
        )
    {
        state.render_revision = state.render_revision.wrapping_add(1);
    }
    if super::tail_actions::handles_tail_action(&action) {
        return reduce_tail(state, action).unwrap_or_default();
    }
    match action {
        Action::Noop => Vec::new(),
        Action::QuitRequested => {
            state.should_quit = true;
            vec![Effect::RequestQuit]
        }
        Action::OverlayClosed | Action::CancelReview => {
            state.overlay = None;
            vec![Effect::None]
        }
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
            state.overlay = Some(Overlay::Input(InputState {
                purpose: crate::overlay::input::InputPurpose::FindQuery,
                ..Default::default()
            }));
            vec![Effect::None]
        }
        Action::DiffChooserOpened => {
            if state
                .selection
                .and_then(|selection| selection.snapshot)
                .is_none()
            {
                "Select an exact snapshot before starting a diff."
                    .clone_into(&mut state.shell_message);
                state.overlay = None;
                return vec![Effect::None];
            }
            state.overlay = Some(Overlay::Input(crate::overlay::input::InputState {
                purpose: crate::overlay::input::InputPurpose::DiffSnapshot,
                ..Default::default()
            }));
            vec![Effect::None]
        }
        Action::ProjectionOpened => {
            let Some(selection) = state.selection else {
                "Select an entity before opening projection.".clone_into(&mut state.shell_message);
                return vec![Effect::None];
            };
            let Some(snapshot) = selection.snapshot else {
                "Projection requires an exact snapshot.".clone_into(&mut state.shell_message);
                return vec![Effect::None];
            };
            reduce(
                state,
                Action::ProjectionRequested {
                    request: super::project_source_request::project_source_request(
                        snapshot,
                        selection.entity,
                        state.project.as_deref(),
                    ),
                },
            )
        }
        Action::InputChanged(_) | Action::Paste(_) | Action::InputSubmitted => {
            reduce_input(state, action)
        }
        Action::InspectRequested {
            request_id,
            entity,
            snapshot,
            switch_view,
        } => {
            if switch_view {
                state.primary_view = crate::selection::PrimaryView::Overview;
            }
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
                if let Some(root) = state.explorer.root.as_mut() {
                    if root.selection.entity == model.entity {
                        if let Some(label) = model.label.clone() {
                            root.label = label;
                        }
                        root.kind.clone_from(&model.kind);
                    }
                }
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
            if category == "transport_unavailable" {
                state.connection = ConnectionState::Disconnected;
                state.stale = true;
            }
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
        Action::ViewSelected(view) => {
            state.primary_view = view;
            state.semantic_row_cursor = None;
            if view == crate::selection::PrimaryView::Session {
                state.cursor = match (&state.session, state.session_cursor) {
                    (LoadState::Ready(model), Some(identity)) => {
                        model.index_of(identity).unwrap_or(0)
                    }
                    _ => 0,
                };
            } else {
                state.cursor = 0;
                state.semantic_row_cursor = None;
            }
            crate::runtime::reconcile_semantic_row_cursor(state);
            if view == crate::selection::PrimaryView::Plan {
                crate::runtime::reconcile_plan_cursor(state);
            }
            if view == crate::selection::PrimaryView::Operations {
                return vec![Effect::ListOperations];
            }
            if view == crate::selection::PrimaryView::Session {
                if let Some(selection) = state.selection {
                    return reduce(
                        state,
                        Action::SessionRequested {
                            session: selection.entity,
                            branch: None,
                        },
                    );
                }
                "Select a session entity to load it.".clone_into(&mut state.shell_message);
            }
            vec![Effect::None]
        }
        Action::HistorySucceeded { request_id, model } => {
            if matches!(state.history, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.history = LoadState::Ready(model);
                crate::runtime::reconcile_semantic_row_cursor(state);
            }
            vec![Effect::None]
        }
        Action::FindSucceeded { request_id, model } => {
            if matches!(state.find, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.find = LoadState::Ready(model);
                crate::runtime::reconcile_semantic_row_cursor(state);
            }
            vec![Effect::None]
        }
        Action::DiffSucceeded { request_id, model } => {
            if matches!(state.diff, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.diff = LoadState::Ready(model);
                crate::runtime::reconcile_semantic_row_cursor(state);
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
                state.cursor = 0;
                state.operation_error = None;
            }
            vec![Effect::None]
        }
        action @ Action::CommitSucceeded { .. } => commit_succeeded(state, action),
        Action::OperationFailed {
            request_id,
            category,
            message,
        } => state_reducer_failure::operation_failed(state, request_id, &category, &message),
        Action::ConfirmReview => {
            if let Some(Overlay::Confirm(confirm)) = state.overlay.take() {
                return reduce(state, Action::CommitConfirmed(confirm.commit));
            }
            if let LoadState::Ready(model) = &state.review {
                if !model.conflicts.is_empty()
                    || model.validation != crate::view::changes::ReviewValidation::Passed
                {
                    "Review conflicts and passing validation are required before confirmation."
                        .clone_into(&mut state.shell_message);
                    return vec![Effect::None];
                }
                let Some(candidate_snapshot) = model.candidate_snapshot else {
                    "The reviewed candidate has no exact result snapshot."
                        .clone_into(&mut state.shell_message);
                    return vec![Effect::None];
                };
                let commit = crate::view::changes::ConfirmedCommit {
                    base_snapshot: model.base_snapshot,
                    candidate_snapshot,
                    change: model.change.clone(),
                    branch: model.branch.clone(),
                };
                state.overlay = Some(Overlay::Confirm(crate::overlay::ConfirmationState {
                    action: "commit change".to_owned(),
                    target: model.change.clone(),
                    base: model.base_snapshot.to_string(),
                    summary: format!(
                        "Candidate {} · {} affected semantic objects",
                        candidate_snapshot,
                        model.deltas.len()
                    ),
                    commit,
                }));
            }
            vec![Effect::None]
        }
        Action::CommitConfirmed(commit) => {
            let LoadState::Ready(model) = &state.review else {
                "No current change review to confirm.".clone_into(&mut state.shell_message);
                return vec![Effect::None];
            };
            if !model.conflicts.is_empty()
                || model.validation != crate::view::changes::ReviewValidation::Passed
                || model.base_snapshot != commit.base_snapshot
                || model.candidate_snapshot != Some(commit.candidate_snapshot)
                || model.change != commit.change
                || model.branch != commit.branch
            {
                "The change review is incomplete or no longer matches this confirmation."
                    .clone_into(&mut state.shell_message);
                state.overlay = None;
                return vec![Effect::None];
            }
            state.overlay = None;
            if let LoadState::Ready(model) = &state.review {
                state.review_proposal = Some(model.clone());
            }
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.review = LoadState::Loading { request_id };
            state.review_request = Some(scriberig_app::ApplyRequest {
                base_snapshot: commit.base_snapshot,
                branch: commit.branch.clone(),
                change: commit.change.clone(),
            });
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
        Action::FindRequested(text) => {
            let Some(snapshot) = state.selection.and_then(|selection| selection.snapshot) else {
                "Find requires an exact snapshot selection.".clone_into(&mut state.shell_message);
                return vec![Effect::None];
            };
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::Find;
            state.find = LoadState::Loading { request_id };
            vec![Effect::RequestFind {
                request_id,
                text,
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
            state.primary_view = crate::selection::PrimaryView::Changes;
            crate::state::state_actions::request_candidate_review(state, request)
        }
        Action::ConnectionRestored => connection_restored::reduce(state),
        _ => state_reducer_tail::reduce_tail(state, action).unwrap_or_default(),
    }
}
