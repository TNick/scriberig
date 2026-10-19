//! Handles TUI actions that activate views and application events.

#![allow(clippy::wildcard_imports)]

use super::activate_palette_control;
use super::*;
#[path = "state_actions/application_event.rs"]
mod application_event;
pub(super) use application_event::reduce_application_event;
#[path = "state_actions/agent_activity.rs"]
mod agent_activity;
#[path = "state_actions/session_changed.rs"]
mod session_changed;

/// Activates the selected local palette item without invoking semantic services.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state containing the selected item.
///
/// Effects produced by the activation.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
pub(super) fn activate_current(state: &mut TuiState) -> Vec<Effect> {
    if state.overlay.is_none() {
        if state.focus == crate::state::Focus::Explorer {
            let selection = if state.cursor == 0 {
                state.explorer.root.as_ref().map(|row| row.selection)
            } else {
                state
                    .explorer
                    .rows
                    .get(state.cursor.saturating_sub(1))
                    .map(|row| row.selection)
            };
            if let Some(selection) = selection {
                if state.cursor == 0
                    && (state.explorer.rows.is_empty() || state.explorer.next.is_some())
                    && let Some(snapshot) = selection.snapshot
                {
                    let request_id = state.next_request_id;
                    state.next_request_id += 1;
                    return vec![Effect::ListExplorer {
                        request_id,
                        entity: selection.entity,
                        snapshot,
                        continuation: state.explorer.next.clone(),
                    }];
                }
                let request_id = state.next_request_id;
                state.next_request_id += 1;
                state.primary_view = crate::selection::PrimaryView::Overview;
                return reduce(
                    state,
                    Action::InspectRequested {
                        request_id,
                        entity: selection.entity,
                        snapshot: selection.snapshot,
                    },
                );
            }
        }
        if state.primary_view == crate::selection::PrimaryView::Operations {
            if let Some(operation) = state.operations.get(state.cursor) {
                if let Some(handle) = operation.result_handles.first() {
                    state
                        .navigation
                        .activate(crate::selection::NavigationLocation {
                            selection: Some(crate::selection::SemanticReference(handle.clone())),
                            ..Default::default()
                        });
                }
            }
        }
        if state.primary_view == crate::selection::PrimaryView::Find
            && let LoadState::Ready(model) = &state.find
            && let Some(row) = model.rows.get(state.cursor)
        {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            return reduce(
                state,
                Action::InspectRequested {
                    request_id,
                    entity: row.entity,
                    snapshot: Some(row.snapshot),
                },
            );
        }
        if state.primary_view == crate::selection::PrimaryView::History
            && let LoadState::Ready(model) = &state.history
            && let Some((entity, _, snapshot)) = model.selection(state.cursor, 0)
        {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            return reduce(
                state,
                Action::InspectRequested {
                    request_id,
                    entity,
                    snapshot: Some(snapshot),
                },
            );
        }
        if state.primary_view == crate::selection::PrimaryView::Diff
            && let LoadState::Ready(model) = &state.diff
            && let Some(row) = model.rows.get(state.cursor)
        {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            return reduce(
                state,
                Action::InspectRequested {
                    request_id,
                    entity: row.entity,
                    snapshot: Some(model.right),
                },
            );
        }
        if state.primary_view == crate::selection::PrimaryView::Changes
            && matches!(state.review, LoadState::Ready(_))
        {
            return reduce(state, Action::ConfirmReview);
        }
        if state.primary_view == crate::selection::PrimaryView::Session {
            if let LoadState::Ready(model) = &state.session {
                if let Some(row) = model.rows.get(state.cursor) {
                    if row.kind == crate::view::session::SessionRowKind::Candidate {
                        let (Some(candidate), Some(base_snapshot)) =
                            (row.candidate.clone(), row.snapshot)
                        else {
                            "Candidate review is missing its exact base snapshot."
                                .clone_into(&mut state.shell_message);
                            return vec![Effect::None];
                        };
                        let request_id = state.next_request_id;
                        state.next_request_id += 1;
                        state.primary_view = crate::selection::PrimaryView::Changes;
                        state.review = LoadState::Loading { request_id };
                        return vec![Effect::Apply {
                            request_id,
                            request: scriberig_app::ApplyRequest {
                                base_snapshot,
                                branch: None,
                                change: candidate,
                            },
                        }];
                    }
                    let id = row.reference;
                    if row.kind == crate::view::session::SessionRowKind::Branch {
                        if let Some(session) =
                            state.agent_handles.last().map(|handle| handle.session)
                        {
                            return reduce(
                                state,
                                Action::SessionRequested {
                                    session,
                                    branch: Some(id),
                                },
                            );
                        }
                    }
                    state
                        .navigation
                        .activate(crate::selection::NavigationLocation {
                            selection: Some(crate::selection::SemanticReference(id.to_string())),
                            ..Default::default()
                        });
                }
            }
        }
        return vec![Effect::None];
    }
    let Some(Overlay::Palette(palette)) = state.overlay.as_ref() else {
        return vec![Effect::None];
    };
    let filtered = palette.filtered();
    let Some(item) = filtered.get(palette.selected) else {
        return vec![Effect::None];
    };
    if item.id == "review-candidate" {
        return refresh_candidate_review(state);
    }
    if let Some(effects) = activate_palette_control(state, &item.id) {
        return effects;
    }
    let action = match item.id.as_str() {
        "help" => Action::HelpOpened,
        "refresh" => Action::RefreshRequested,
        "back" => Action::Back,
        "quit" => Action::QuitRequested,
        "inspect_entity" => {
            state.overlay = Some(Overlay::Input(InputState::default()));
            return vec![Effect::None];
        }
        "overview" => Action::ViewSelected(crate::selection::PrimaryView::Overview),
        "history" => {
            let Some(snapshot) = state.selection.and_then(|selection| selection.snapshot) else {
                return vec![Effect::None];
            };
            state.overlay = None;
            return reduce(state, Action::HistoryRequested { snapshot });
        }
        "diff" => return reduce(state, Action::DiffChooserOpened),
        "projection" => return reduce(state, Action::ProjectionOpened),
        "edit" => {
            return begin_projection_edit(state);
        }
        "project-agent" => {
            let Some(selection) = state.selection else {
                return vec![Effect::None];
            };
            let Some(snapshot) = selection.snapshot else {
                return vec![Effect::None];
            };
            state.overlay = None;
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.primary_view = crate::selection::PrimaryView::Changes;
            state.review = LoadState::Loading { request_id };
            return vec![Effect::ProjectAgent {
                request_id,
                entity: selection.entity,
                snapshot,
                prompt: "Change add so overflow saturates instead of wrapping.".to_owned(),
            }];
        }
        "changes" => {
            "Changes appear after an application proposes a migration."
                .clone_into(&mut state.shell_message);
            state.overlay = None;
            return vec![Effect::None];
        }
        "session" => Action::ViewSelected(crate::selection::PrimaryView::Session),
        "start-agent" => {
            state.overlay = None;
            return reduce(
                state,
                Action::SessionComposerOpened {
                    session: state
                        .selection
                        .map_or_else(scriberig_core::EntityId::new, |selection| selection.entity),
                    branch: None,
                },
            );
        }
        "continue-agent" => {
            let Some(session) = state.agent_handles.last().map(|handle| handle.session) else {
                "Select a session before continuing a branch.".clone_into(&mut state.shell_message);
                state.overlay = None;
                return vec![Effect::None];
            };
            let branch = match &state.session {
                LoadState::Ready(model) => model
                    .rows
                    .get(state.cursor)
                    .filter(|row| row.kind == crate::view::session::SessionRowKind::Branch)
                    .map(|row| row.reference),
                _ => None,
            };
            let Some(branch) = branch else {
                "Select a branch before continuing it.".clone_into(&mut state.shell_message);
                state.overlay = None;
                return vec![Effect::None];
            };
            state.overlay = None;
            return reduce(
                state,
                Action::SessionComposerOpened {
                    session,
                    branch: Some(branch),
                },
            );
        }
        "operations" => Action::ViewSelected(crate::selection::PrimaryView::Operations),
        _ => return vec![Effect::None],
    };
    state.overlay = None;
    reduce(state, action)
}

/// Refreshes the selected session before opening its newest candidate review.
///
/// - `state`: TUI state containing the selected session handle.
///
/// Session refresh effect, or a local candidate review result.
/// # Returns
/// The effect that refreshes the session or opens the latest candidate review.
pub(super) fn refresh_candidate_review(state: &mut TuiState) -> Vec<Effect> {
    state.primary_view = crate::selection::PrimaryView::Changes;
    state.focus = crate::state::Focus::Primary;
    let Some(session) = state.agent_handles.last().map(|handle| handle.session) else {
        return review_latest_candidate(state);
    };
    let request_id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    state.session = LoadState::Loading { request_id };
    vec![Effect::ReadSession {
        request_id,
        session,
        branch: None,
    }]
}

/// Opens the newest candidate review recorded in the selected session.
///
/// - `state`: TUI state containing the selected session and current overlay.
///
/// Apply preview effect, or a local message when no candidate is available.
/// # Returns
/// The effect that opens the latest candidate preview, or no effect.
pub(super) fn review_latest_candidate(state: &mut TuiState) -> Vec<Effect> {
    let candidate = match &state.session {
        LoadState::Ready(model) => model
            .rows
            .iter()
            .rev()
            .find_map(|row| Some((row.candidate.clone()?, row.snapshot?))),
        _ => None,
    };
    let Some((candidate, base_snapshot)) = candidate else {
        state.overlay = None;
        state.primary_view = crate::selection::PrimaryView::Changes;
        "No durable project candidate is available in the selected session."
            .clone_into(&mut state.shell_message);
        return vec![Effect::None];
    };
    state.overlay = None;
    let request_id = state.next_request_id;
    state.next_request_id += 1;
    state.primary_view = crate::selection::PrimaryView::Changes;
    state.review = LoadState::Loading { request_id };
    vec![Effect::Apply {
        request_id,
        request: scriberig_app::ApplyRequest {
            base_snapshot,
            branch: None,
            change: candidate,
        },
    }]
}

/// Opens the semantic source edit input for the selected projection.
///
/// `state`: TUI state containing the selected entity and active view.
///
/// A no-op effect after opening the editor or reporting its prerequisite.
/// # Returns
/// The reducer effect produced by the state change.
pub(super) fn begin_projection_edit(state: &mut TuiState) -> Vec<Effect> {
    if state
        .selection
        .is_none_or(|selection| selection.snapshot.is_none())
    {
        "Select a source at an exact snapshot before editing it."
            .clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    }
    state.primary_view = crate::selection::PrimaryView::Projection;
    state.overlay = Some(Overlay::Input(crate::overlay::input::InputState {
        purpose: crate::overlay::input::InputPurpose::ProjectionEdit,
        ..Default::default()
    }));
    vec![Effect::None]
}
