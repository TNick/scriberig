//! Covers additional focused cases for the parent test module.

use super::*;

/// Rebinds the next reply to authoritative session and branch handles.
#[test]
fn accepted_conversation_start_installs_shared_branch_head_anchor() {
    let mut state = TuiState::default();
    let session = scriberig_core::EntityId::from_bytes([71; 16]);
    let branch = scriberig_core::EntityId::from_bytes([72; 16]);
    let operation = scriberig_core::OperationId::new("conversation-accepted").expect("operation");

    let effects = reduce(
        &mut state,
        Action::ConversationAgentAccepted(crate::service::AgentHandle {
            operation: operation.clone(),
            session,
            branch: Some(branch),
            workspace: None,
            failure: None,
        }),
    );

    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::Existing {
            session,
            reply_anchor: scriberig_session::ContinuationAnchor::BranchHead { branch },
        }
    );
    assert!(matches!(
        effects.as_slice(),
        [Effect::ReadSession { session: value, .. }] if *value == session
    ));
    assert_eq!(state.agent_handles[0].operation, operation);
}

/// Verifies conversation cursor survives structural rerender.
#[test]
fn conversation_cursor_survives_structural_rerender() {
    let session_id = scriberig_core::EntityId::from_bytes([81; 16]);
    let root_id = scriberig_core::EntityId::from_bytes([82; 16]);
    let first_turn = scriberig_core::EntityId::from_bytes([83; 16]);
    let target_turn = scriberig_core::EntityId::from_bytes([84; 16]);
    let mut root = scriberig_session::Branch::new(root_id, "Root").expect("root");
    for (id, ordinal) in [(first_turn, 0), (target_turn, 1)] {
        root.push_turn(scriberig_session::Turn {
            id,
            ordinal,
            human: Some(scriberig_session::HumanInput {
                text: format!("message {ordinal}"),
            }),
            agent: None,
            tool_invocation: None,
            tool_result: None,
        });
    }
    let session = scriberig_session::Session {
        id: session_id,
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let target = crate::view::conversation::ConversationSource::Activity(target_turn);
    let mut state = TuiState {
        conversation_cursor: Some(target),
        ..TuiState::default()
    };
    state.conversation_composer.target = crate::state::ConversationTarget::Existing {
        session: session_id,
        reply_anchor: scriberig_session::ContinuationAnchor::HumanInput {
            branch: root_id,
            turn: target_turn,
        },
    };
    state.session = crate::inspector::LoadState::Loading { request_id: 4 };
    let mut updated = session.clone();
    updated.branches.insert(
        0,
        scriberig_session::Branch::new(
            scriberig_core::EntityId::from_bytes([80; 16]),
            "New sibling",
        )
        .expect("sibling"),
    );
    let conversation = crate::view::conversation::ConversationModel::from_session(&updated);
    let model = crate::view::session::SessionModel::from_session(&updated);

    reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 4,
            model,
            conversation,
            session_snapshot: Some(updated),
        },
    );

    assert_eq!(state.conversation_cursor, Some(target));
    assert_eq!(state.cursor, 4);
    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::Existing {
            session: session_id,
            reply_anchor: scriberig_session::ContinuationAnchor::HumanInput {
                branch: root_id,
                turn: target_turn,
            },
        }
    );
}

/// Keeps structural plan rows from becoming accidental continuation targets.
#[test]
fn conversation_non_replyable_row_cannot_be_pinned() {
    let mut state = TuiState::default();
    let plan = scriberig_core::EntityId::from_bytes([99; 16]);
    state.conversation = Some(crate::view::conversation::ConversationModel {
        rows: vec![crate::view::conversation::ConversationRow {
            source: crate::view::conversation::ConversationSource::Plan(plan),
            reply_anchor: None,
            depth: 0,
            label: "Plan: inspect".to_owned(),
            kind: crate::view::conversation::ConversationRowKind::Plan,
        }],
        coordinator_cursor: None,
    });
    state.cursor = 0;

    let effects = reduce(&mut state, Action::ConversationPinCursor);

    assert!(matches!(effects.as_slice(), [Effect::None]));
    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::NewSession
    );
    assert!(state.shell_message.contains("cannot receive a reply"));
}

/// Pins a historical human turn using its exact durable continuation anchor.
#[test]
fn historical_replyable_activity_can_be_pinned_exactly() {
    let session_id = scriberig_core::EntityId::from_bytes([134; 16]);
    let branch_id = scriberig_core::EntityId::from_bytes([135; 16]);
    let first_turn = scriberig_core::EntityId::from_bytes([136; 16]);
    let target_turn = scriberig_core::EntityId::from_bytes([137; 16]);
    let mut branch = scriberig_session::Branch::new(branch_id, "Root").expect("root");
    for (id, ordinal) in [(first_turn, 0), (target_turn, 1)] {
        branch.push_turn(scriberig_session::Turn {
            id,
            ordinal,
            human: Some(scriberig_session::HumanInput {
                text: format!("message {ordinal}"),
            }),
            agent: None,
            tool_invocation: None,
            tool_result: None,
        });
    }
    let session = scriberig_session::Session {
        id: session_id,
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let projection = crate::view::conversation::ConversationModel::from_session(&session);
    let mut state = TuiState {
        session_snapshot: Some(session),
        conversation: Some(projection),
        ..TuiState::default()
    };
    state.cursor = state
        .conversation
        .as_ref()
        .and_then(|model| {
            model.index_of(crate::view::conversation::ConversationSource::Activity(
                target_turn,
            ))
        })
        .expect("historical turn row");

    reduce(&mut state, Action::ConversationPinCursor);

    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::Existing {
            session: session_id,
            reply_anchor: scriberig_session::ContinuationAnchor::HumanInput {
                branch: branch_id,
                turn: target_turn,
            },
        }
    );
}

/// Keeps user reply targeting separate from a moving plan coordinator cursor.
#[test]
fn coordinator_follow_and_pinned_reply_anchor_remain_independent() {
    let session = scriberig_core::EntityId::from_bytes([91; 16]);
    let plan = scriberig_core::EntityId::from_bytes([92; 16]);
    let coordinator_node = scriberig_core::EntityId::from_bytes([93; 16]);
    let branch = scriberig_core::EntityId::from_bytes([94; 16]);
    let turn = scriberig_core::EntityId::from_bytes([95; 16]);
    let mut source = scriberig_session::Branch::new(branch, "Root").expect("root");
    source.push_turn(scriberig_session::Turn {
        id: turn,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "question".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "answer".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    let session_value = scriberig_session::Session {
        id: session,
        branches: vec![source],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut conversation =
        crate::view::conversation::ConversationModel::from_session(&session_value);
    conversation.coordinator_cursor = Some((plan, coordinator_node));
    let reply_anchor = scriberig_session::ContinuationAnchor::AssistantOutput { branch, turn };
    let mut state = TuiState {
        conversation: Some(conversation),
        conversation_composer: crate::state::ConversationComposerState {
            target: crate::state::ConversationTarget::Existing {
                session,
                reply_anchor,
            },
            draft: String::new(),
            editor: scriberig_tui_kit::Composer::new(),
            pending_prompt: None,
            pending: false,
            error: None,
        },
        ..TuiState::default()
    };
    reduce(&mut state, Action::ConversationFollowCoordinator);
    assert_eq!(state.coordinator_cursor, Some((plan, coordinator_node)));
    state.conversation_browsing = true;
    reduce(
        &mut state,
        Action::CursorMoved(crate::action::Direction::Next),
    );

    assert!(!state.follow_coordinator);
    assert_eq!(state.coordinator_cursor, Some((plan, coordinator_node)));
    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::Existing {
            session,
            reply_anchor,
        }
    );
}

/// Expands completed activity locally without changing anchor or shared session.
#[test]
fn expansion_and_layout_changes_preserve_reply_anchor() {
    let session_id = scriberig_core::EntityId::from_bytes([101; 16]);
    let branch = scriberig_core::EntityId::from_bytes([102; 16]);
    let turn = scriberig_core::EntityId::from_bytes([103; 16]);
    let mut root = scriberig_session::Branch::new(branch, "Root").expect("root");
    root.push_turn(scriberig_session::Turn {
        id: turn,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "question".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "answer".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    root.transition(scriberig_session::BranchStatus::Completed)
        .expect("completed");
    let session = scriberig_session::Session {
        id: session_id,
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let anchor = scriberig_session::ContinuationAnchor::AssistantOutput { branch, turn };
    let mut state = TuiState {
        session_snapshot: Some(session.clone()),
        conversation: Some(crate::view::conversation::ConversationModel::from_session(
            &session,
        )),
        conversation_cursor: Some(crate::view::conversation::ConversationSource::Branch(
            branch,
        )),
        conversation_composer: crate::state::ConversationComposerState {
            target: crate::state::ConversationTarget::Existing {
                session: session_id,
                reply_anchor: anchor,
            },
            draft: String::new(),
            editor: scriberig_tui_kit::Composer::new(),
            pending_prompt: None,
            pending: false,
            error: None,
        },
        ..TuiState::default()
    };

    assert_eq!(
        reduce(&mut state, Action::ConversationToggleExpansion),
        vec![Effect::None]
    );
    state.terminal_size = (42, 15);
    state.tool_output_visibility = crate::state::ToolOutputVisibility::Full;
    state.reasoning_visibility = crate::state::ReasoningVisibility::Shown;

    assert!(state.expanded_branches.contains(&branch));
    assert_eq!(state.session_snapshot, Some(session));
    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::Existing {
            session: session_id,
            reply_anchor: anchor,
        }
    );
}

#[path = "state_session_composer_tests_part1_part2.rs"]
mod part2;
