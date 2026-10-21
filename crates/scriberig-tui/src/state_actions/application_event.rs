//! Reduces shared application lifecycle events into TUI state.

use super::*;

/// Converts shared events into bounded local operation observations.
///
/// - `state`: TUI state to update.
/// - `event`: Application event to observe.
///
/// Effects produced by the observation.
/// # Returns
/// The operation result.
pub(in crate::state) fn reduce_application_event(
    state: &mut TuiState,
    event: &scriberig_app::ApplicationEvent,
) -> Vec<Effect> {
    match event {
        scriberig_app::ApplicationEvent::SessionChanged { session } => {
            return super::session_changed::refresh_active_session(state, *session);
        }
        scriberig_app::ApplicationEvent::AgentOutputProduced { operation, output } => {
            return reduce(
                state,
                Action::AgentOutputObserved {
                    operation: operation.clone(),
                    text: output.clone(),
                    complete: true,
                },
            );
        }
        scriberig_app::ApplicationEvent::AgentOutputDelta { operation, text } => {
            return reduce(
                state,
                Action::AgentOutputObserved {
                    operation: operation.clone(),
                    text: text.clone(),
                    complete: false,
                },
            );
        }
        scriberig_app::ApplicationEvent::AgentActivity {
            operation,
            branch,
            activity_kind,
            state: activity_state,
            summary,
            ..
        } => {
            return super::agent_activity::observe_agent_activity(
                state,
                &operation.to_string(),
                branch,
                *activity_kind,
                *activity_state,
                summary,
            );
        }
        scriberig_app::ApplicationEvent::SessionSelectionChanged {
            activity_kind,
            summary,
            ..
        } => {
            let effects = super::agent_activity::observe_agent_activity(
                state,
                &format!("session-selection-{activity_kind:?}"),
                "session",
                *activity_kind,
                scriberig_session::AgentActivityState::Completed,
                summary,
            );
            state.shell_message.clone_from(summary);
            return effects;
        }
        _ => {}
    }
    let previous = match event {
        scriberig_app::ApplicationEvent::OperationStarted { operation }
        | scriberig_app::ApplicationEvent::OperationProgressed { operation, .. }
        | scriberig_app::ApplicationEvent::OperationCompleted { operation }
        | scriberig_app::ApplicationEvent::OperationFailed { operation, .. }
        | scriberig_app::ApplicationEvent::OperationCancelled { operation } => {
            state.operations.iter().find(|value| &value.id == operation)
        }
        _ => return vec![Effect::None],
    };
    let Some(model) = crate::view::operations::OperationModel::from_event(event, previous) else {
        return vec![Effect::None];
    };
    let needs_detail = previous.is_none() && !model.status.is_terminal();
    let operation = model.id.clone();
    let mut effects = reduce(state, Action::OperationObserved(model));
    if needs_detail && !state.pending_operation_refresh.contains(&operation) {
        state.pending_operation_refresh.push(operation.clone());
        effects.extend(reduce(state, Action::OperationStatusRequested(operation)));
    }
    effects
}

#[cfg(test)]
#[path = "application_event_tests.rs"]
mod tests;
