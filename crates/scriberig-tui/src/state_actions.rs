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
    if let scriberig_app::ApplicationEvent::AgentOutputProduced { output } = event {
        return reduce(state, Action::AgentOutputObserved(output.clone()));
    }
    let operation = match event {
        scriberig_app::ApplicationEvent::OperationStarted { operation }
        | scriberig_app::ApplicationEvent::OperationProgressed { operation, .. }
        | scriberig_app::ApplicationEvent::OperationCompleted { operation } => operation.clone(),
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
pub(super) fn activate_current(state: &mut TuiState) -> Vec<Effect> {
    if state.overlay.is_none() {
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
        if state.primary_view == crate::selection::PrimaryView::Session {
            if let LoadState::Ready(model) = &state.session {
                if let Some(row) = model.rows.get(state.cursor) {
                    let id = row.reference;
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
            return reduce(state, Action::HistoryRequested { snapshot });
        }
        "diff" | "projection" | "changes" => {
            return vec![Effect::None];
        }
        "session" => Action::ViewSelected(crate::selection::PrimaryView::Session),
        "operations" => Action::ViewSelected(crate::selection::PrimaryView::Operations),
        _ => return vec![Effect::None],
    };
    state.overlay = None;
    reduce(state, action)
}
