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
        scriberig_app::ApplicationEvent::AgentOutputProduced { output, .. } => {
            return reduce(state, Action::AgentOutputObserved(output.clone()));
        }
        scriberig_app::ApplicationEvent::AgentOutputDelta { text, .. } => {
            return reduce(state, Action::AgentOutputObserved(text.clone()));
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
mod tests {
    use super::reduce_application_event;
    use crate::{
        action::Action,
        inspector::LoadState,
        state::TuiState,
        view::{conversation::ConversationSource, session::SessionModel},
    };

    /// Keeps selection changes visible even when reasoning rows are hidden.
    #[test]
    fn selection_activity_is_announced_independently_of_reasoning_visibility() {
        let mut state = TuiState::default();
        reduce_application_event(
            &mut state,
            &scriberig_app::ApplicationEvent::SessionSelectionChanged {
                session: scriberig_core::EntityId::new(),
                activity_kind: scriberig_session::AgentActivityKind::ModelSelectionChanged,
                summary: "Model selected: Model A".to_owned(),
            },
        );

        assert_eq!(state.shell_message, "Model selected: Model A");
        assert_eq!(state.transient_agent_activities.len(), 1);
    }

    /// Reconstructs one structural live row and clears it after settlement.
    #[test]
    #[allow(clippy::too_many_lines)] // The scenario verifies durable refresh and deduplication together.
    fn live_worker_activity_is_structural_and_deduplicated_after_refresh() {
        let session_id = scriberig_core::EntityId::from_bytes([171; 16]);
        let branch_id = scriberig_core::EntityId::from_bytes([172; 16]);
        let second_branch = scriberig_core::EntityId::from_bytes([173; 16]);
        let root_branch = scriberig_core::EntityId::from_bytes([174; 16]);
        let operation =
            scriberig_core::OperationId::new("worker-operation").expect("operation identity");
        let second_operation = scriberig_core::OperationId::new("second-worker-operation")
            .expect("second operation identity");
        let session = scriberig_session::Session {
            id: session_id,
            branches: {
                let mut root =
                    scriberig_session::Branch::new(root_branch, "Root").expect("root branch");
                root.children = vec![branch_id, second_branch];
                vec![
                    root,
                    scriberig_session::Branch::new_child(
                        branch_id,
                        root_branch,
                        "Worker one",
                        "first task",
                    )
                    .expect("first branch"),
                    scriberig_session::Branch::new_child(
                        second_branch,
                        root_branch,
                        "Worker two",
                        "second task",
                    )
                    .expect("second branch"),
                ]
            },
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        };
        let activity =
            |operation: &scriberig_core::OperationId,
             branch: scriberig_core::EntityId,
             state,
             summary: &str| scriberig_app::ApplicationEvent::AgentActivity {
                operation: operation.clone(),
                branch: branch.to_string(),
                activity_kind: scriberig_session::AgentActivityKind::RuntimeStatus,
                state,
                summary: summary.to_owned(),
            };
        let mut state = TuiState {
            session: LoadState::Loading { request_id: 3 },
            ..TuiState::default()
        };
        let reply_target = crate::state::ConversationTarget::Existing {
            session: session_id,
            reply_anchor: scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        };
        state.conversation_composer.target = reply_target;

        reduce_application_event(
            &mut state,
            &activity(
                &operation,
                branch_id,
                scriberig_session::AgentActivityState::Running,
                "working",
            ),
        );
        reduce_application_event(
            &mut state,
            &activity(
                &operation,
                branch_id,
                scriberig_session::AgentActivityState::Running,
                "working",
            ),
        );
        reduce_application_event(
            &mut state,
            &activity(
                &second_operation,
                second_branch,
                scriberig_session::AgentActivityState::Running,
                "independent work",
            ),
        );
        crate::state::reduce(
            &mut state,
            Action::SessionSucceeded {
                request_id: 3,
                model: SessionModel::from_session(&session),
                conversation: crate::view::conversation::ConversationModel::from_session(&session),
                session_snapshot: Some(session.clone()),
            },
        );

        let live_rows = state
            .conversation
            .as_ref()
            .expect("conversation tree")
            .rows
            .iter()
            .filter(|row| matches!(row.source, ConversationSource::TransientActivity(_)))
            .collect::<Vec<_>>();
        assert_eq!(live_rows.len(), 2);
        for (branch, label) in [(branch_id, "working"), (second_branch, "independent work")] {
            let live = live_rows
                .iter()
                .find(|row| row.label.contains(label))
                .expect("worker status row");
            let rows = &state.conversation.as_ref().expect("conversation tree").rows;
            let branch_index = rows
                .iter()
                .position(|row| row.source == ConversationSource::Branch(branch))
                .expect("worker branch");
            let live_index = rows
                .iter()
                .position(|row| row.source == live.source)
                .expect("attached status row");
            assert!(live_index > branch_index);
        }
        reduce_application_event(
            &mut state,
            &activity(
                &operation,
                branch_id,
                scriberig_session::AgentActivityState::Running,
                "working again",
            ),
        );
        let updated_live_rows = state
            .conversation
            .as_ref()
            .expect("live conversation tree")
            .rows
            .iter()
            .filter(|row| matches!(row.source, ConversationSource::TransientActivity(_)))
            .collect::<Vec<_>>();
        assert_eq!(updated_live_rows.len(), 2);
        assert!(
            updated_live_rows
                .iter()
                .any(|row| row.label.contains("working again"))
        );
        assert_eq!(state.conversation_composer.target, reply_target);

        reduce_application_event(
            &mut state,
            &activity(
                &operation,
                branch_id,
                scriberig_session::AgentActivityState::Completed,
                "done",
            ),
        );
        state.session = LoadState::Loading { request_id: 4 };
        crate::state::reduce(
            &mut state,
            Action::SessionSucceeded {
                request_id: 4,
                model: SessionModel::from_session(&session),
                conversation: crate::view::conversation::ConversationModel::from_session(&session),
                session_snapshot: Some(session),
            },
        );
        assert_eq!(state.transient_agent_activities.len(), 1);
        let remaining = state
            .conversation
            .as_ref()
            .expect("refreshed conversation")
            .rows
            .iter()
            .filter(|row| matches!(row.source, ConversationSource::TransientActivity(_)))
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), 1);
        assert!(remaining[0].label.contains("independent work"));
        assert_eq!(state.conversation_composer.target, reply_target);
    }
}
