//! Reduces shared application lifecycle events into TUI state.

use super::*;

/// Converts shared events into bounded local operation observations.
///
/// # Parameters
/// - `state`: TUI state to update.
/// - `event`: Application event to observe.
///
/// # Returns
/// Effects produced by the observation.
pub(in crate::state) fn reduce_application_event(
    state: &mut TuiState,
    event: &scriberig_app::ApplicationEvent,
) -> Vec<Effect> {
    match event {
        scriberig_app::ApplicationEvent::SessionChanged { session } => {
            return super::session_changed::refresh_active_session(state, *session);
        }
        scriberig_app::ApplicationEvent::AgentOutputProduced { output, .. } => {
            return reduce(state, Action::AgentOutputObserved(output.clone()));
        }
        scriberig_app::ApplicationEvent::AgentOutputDelta { text, .. } => {
            return reduce(state, Action::AgentOutputObserved(text.clone()));
        }
        scriberig_app::ApplicationEvent::AgentActivity {
            branch,
            activity_kind,
            state: activity_state,
            summary,
            ..
        } => {
            return super::agent_activity::observe_agent_activity(
                state,
                branch,
                activity_kind,
                activity_state,
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
                "session",
                activity_kind,
                "completed",
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
mod tests {
    use super::reduce_application_event;
    use crate::state::TuiState;

    /// Keeps selection changes visible even when reasoning rows are hidden.
    #[test]
    fn selection_activity_is_announced_independently_of_reasoning_visibility() {
        let mut state = TuiState::default();
        reduce_application_event(
            &mut state,
            &scriberig_app::ApplicationEvent::SessionSelectionChanged {
                session: scriberig_core::EntityId::new(),
                activity_kind: "modelSelection".to_owned(),
                summary: "Model selected: Model A".to_owned(),
            },
        );

        assert_eq!(state.shell_message, "Model selected: Model A");
        assert_eq!(state.transient_agent_activities.len(), 1);
    }
}
