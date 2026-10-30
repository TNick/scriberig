//! Covers terminal lifecycle recovery behavior.

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

/// Replaces streamed fragments with durable output at the accepted branch.
#[test]
#[allow(clippy::too_many_lines)] // Covers fragment accumulation and durable handoff.
fn streamed_output_is_operation_scoped_and_yields_to_durable_text() {
    let session_id = scriberig_core::EntityId::from_bytes([81; 16]);
    let branch_id = scriberig_core::EntityId::from_bytes([82; 16]);
    let operation =
        scriberig_core::OperationId::new("stream-operation").expect("operation identity");
    let branch = scriberig_session::Branch::new(branch_id, "Root").expect("root branch");
    let mut session = scriberig_session::Session {
        id: session_id,
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut state = TuiState {
        agent_handles: vec![crate::service::AgentHandle {
            operation: operation.clone(),
            session: session_id,
            branch: Some(branch_id),
            workspace: None,
            failure: None,
        }],
        conversation_cursor: Some(ConversationSource::Branch(branch_id)),
        conversation_composer: crate::state::ConversationComposerState {
            target: crate::state::ConversationTarget::Existing {
                session: session_id,
                reply_anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: branch_id,
                },
            },
            draft: String::new(),
            pending_prompt: None,
            pending: false,
            error: None,
        },
        ..TuiState::default()
    };

    reduce_application_event(
        &mut state,
        &scriberig_app::ApplicationEvent::AgentOutputDelta {
            operation: operation.clone(),
            text: "Hello".to_owned(),
        },
    );
    reduce_application_event(
        &mut state,
        &scriberig_app::ApplicationEvent::AgentActivity {
            operation: operation.clone(),
            branch: branch_id.to_string(),
            activity_kind: scriberig_session::AgentActivityKind::ToolInvocation,
            state: scriberig_session::AgentActivityState::Running,
            summary: "tool started".to_owned(),
            ordinal: 1,
        },
    );
    reduce_application_event(
        &mut state,
        &scriberig_app::ApplicationEvent::AgentOutputDelta {
            operation: operation.clone(),
            text: " \u{1b}[2Jworld".to_owned(),
        },
    );

    assert_eq!(state.transient_agent_outputs.len(), 1);
    assert_eq!(state.transient_agent_outputs[0].text, "Hello [2Jworld");
    assert_eq!(
        state.conversation_cursor,
        Some(ConversationSource::Branch(branch_id))
    );
    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::Existing {
            session: session_id,
            reply_anchor: scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        }
    );
    let rows = &state
        .conversation
        .as_ref()
        .expect("conversation projection")
        .rows;
    let transient = rows
        .iter()
        .find(|row| matches!(row.source, ConversationSource::TransientActivity(_)))
        .expect("streamed assistant row");
    assert_eq!(transient.label, "Assistant: Hello [2Jworld");
    assert_eq!(
        rows.iter()
            .filter(|row| row.label == "Assistant: Hello [2Jworld")
            .count(),
        1
    );
    let activity_index = rows
        .iter()
        .position(|row| row.label.contains("tool started"))
        .expect("interleaved tool activity");
    let output_index = rows
        .iter()
        .position(|row| row.label == "Assistant: Hello [2Jworld")
        .expect("streamed assistant row");
    assert!(output_index < activity_index);

    reduce_application_event(
        &mut state,
        &scriberig_app::ApplicationEvent::AgentOutputProduced {
            operation: operation.clone(),
            output: "Hello \u{1b}[2Jworld".to_owned(),
        },
    );
    assert_eq!(state.transient_agent_outputs.len(), 1);
    assert!(state.transient_agent_outputs[0].complete);
    reduce_application_event(
        &mut state,
        &scriberig_app::ApplicationEvent::AgentActivity {
            operation: operation.clone(),
            branch: branch_id.to_string(),
            activity_kind: scriberig_session::AgentActivityKind::ToolInvocation,
            state: scriberig_session::AgentActivityState::Completed,
            summary: "tool completed".to_owned(),
            ordinal: 2,
        },
    );

    let turn_id = scriberig_core::EntityId::from_bytes([83; 16]);
    let durable_branch = session.branches.first_mut().expect("root branch");
    durable_branch.push_turn(scriberig_session::Turn {
        id: turn_id,
        ordinal: 0,
        human: None,
        agent: Some(scriberig_session::AgentOutput {
            text: "Hello \u{1b}[2Jworld".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    durable_branch
        .activity
        .push(scriberig_session::ExecutionActivity::AgentOutput(turn_id));
    state.session = LoadState::Loading { request_id: 11 };
    crate::state::reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 11,
            model: SessionModel::from_session(&session),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: Some(session),
        },
    );
    assert_eq!(state.transient_agent_outputs.len(), 0);
    assert!(
        !state
            .conversation
            .as_ref()
            .expect("durable conversation")
            .rows
            .iter()
            .any(|row| matches!(row.source, ConversationSource::TransientActivity(_)))
    );
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
            ordinal: 0,
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
