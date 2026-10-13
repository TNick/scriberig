//! Handles TUI actions that activate views and application events.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Converts shared events into bounded local operation observations.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state to update.
/// - `event`: Application event to observe.
///
/// # Returns
/// Effects produced by the observation.
pub(super) fn reduce_application_event(
    state: &mut TuiState,
    event: &scriberig_app::ApplicationEvent,
) -> Vec<Effect> {
    match event {
        scriberig_app::ApplicationEvent::AgentOutputProduced { output, .. } => {
            return reduce(state, Action::AgentOutputObserved(output.clone()));
        }
        scriberig_app::ApplicationEvent::AgentOutputDelta { text, .. } => {
            return reduce(state, Action::AgentOutputObserved(text.clone()));
        }
        _ => {}
    }
    let operation = match event {
        scriberig_app::ApplicationEvent::OperationStarted { operation }
        | scriberig_app::ApplicationEvent::OperationProgressed { operation, .. }
        | scriberig_app::ApplicationEvent::OperationCompleted { operation }
        | scriberig_app::ApplicationEvent::OperationFailed { operation, .. }
        | scriberig_app::ApplicationEvent::OperationCancelled { operation } => operation.clone(),
        _ => return vec![Effect::None],
    };
    if state.pending_operation_refresh.contains(&operation) {
        return vec![Effect::None];
    }
    state.pending_operation_refresh.push(operation.clone());
    reduce(state, Action::OperationStatusRequested(operation))
}

/// Activates the selected local palette item without invoking semantic services.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state containing the selected item.
///
/// # Returns
/// Effects produced by the activation.
#[allow(clippy::too_many_lines)]
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
                    let id = row.reference;
                    if row.kind == crate::view::session::SessionRowKind::Branch {
                        if let Some(session) = state.selection.map(|value| value.entity) {
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
        "diff" => {
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
            return vec![Effect::None];
        }
        "projection" => {
            state.overlay = None;
            let Some(selection) = state.selection else {
                "Select an entity before opening projection.".clone_into(&mut state.shell_message);
                return vec![Effect::None];
            };
            let Some(snapshot) = selection.snapshot else {
                "Projection requires an exact snapshot.".clone_into(&mut state.shell_message);
                return vec![Effect::None];
            };
            return reduce(
                state,
                Action::ProjectionRequested {
                    request: scriberig_app::RenderRequest {
                        snapshot: scriberig_app::SnapshotSelector::Exact(snapshot),
                        view: format!("project-source:{}", selection.entity),
                    },
                },
            );
        }
        "edit" => {
            if state.selection.is_none()
                || !matches!(
                    state.primary_view,
                    crate::selection::PrimaryView::Projection
                )
            {
                "Open a source projection before editing it.".clone_into(&mut state.shell_message);
                state.overlay = None;
                return vec![Effect::None];
            }
            state.overlay = Some(Overlay::Input(crate::overlay::input::InputState {
                purpose: crate::overlay::input::InputPurpose::ProjectionEdit,
                ..Default::default()
            }));
            return vec![Effect::None];
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
                Action::ComposerOpened {
                    session: state
                        .selection
                        .map_or_else(scriberig_core::EntityId::new, |selection| selection.entity),
                    branch: None,
                },
            );
        }
        "continue-agent" => {
            let Some(session) = state.selection else {
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
                Action::ComposerOpened {
                    session: session.entity,
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
