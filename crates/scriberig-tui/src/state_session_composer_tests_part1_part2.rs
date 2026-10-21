//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies composer slash commands open shared controls.
#[test]
fn composer_slash_commands_open_shared_controls() {
    let mut state = TuiState::default();
    reduce(
        &mut state,
        Action::SessionComposerOpened {
            session: scriberig_core::EntityId::from_bytes([35; 16]),
            branch: None,
        },
    );
    reduce(&mut state, Action::InputChanged("/model".to_owned()));

    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::ReadModelLibrary { .. }]
    ));
    assert_eq!(state.session_composer.as_ref().unwrap().text, "");

    state.model_library = crate::inspector::LoadState::NotRequested;
    reduce(&mut state, Action::InputChanged("/thinking".to_owned()));
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::None]
    ));
    assert!(state.shell_message.contains("Load a session"));

    reduce(
        &mut state,
        Action::InputChanged("/view reasoning".to_owned()),
    );
    assert_eq!(
        reduce(&mut state, Action::InputSubmitted),
        vec![Effect::None]
    );
    assert_eq!(
        state.reasoning_visibility,
        crate::state::ReasoningVisibility::Shown
    );
    assert_eq!(state.session_composer.as_ref().unwrap().text, "");
}

/// Opens the structural session view locally and returns with Escape.
#[test]
fn session_slash_view_returns_to_conversation() {
    let session_id = scriberig_core::EntityId::from_bytes([117; 16]);
    let root_id = scriberig_core::EntityId::from_bytes([118; 16]);
    let session = scriberig_session::Session {
        id: session_id,
        branches: vec![scriberig_session::Branch::new(root_id, "Root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut state = TuiState {
        session: crate::inspector::LoadState::Ready(
            crate::view::session::SessionModel::from_session(&session),
        ),
        session_snapshot: Some(session),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/session".to_owned();

    reduce(&mut state, Action::InputSubmitted);
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Session);
    let escape = crate::keymap::action_for(
        &state,
        &crate::event::Key {
            code: crate::event::KeyCode::Esc,
            modifiers: crate::event::Modifiers::default(),
        },
    )
    .expect("escape returns to conversation");
    reduce(&mut state, escape);

    assert_eq!(
        state.primary_view,
        crate::selection::PrimaryView::Conversation
    );
}

/// Opens snapshot-scoped Explorer through a local slash command.
#[test]
fn explorer_slash_command_lists_the_selected_snapshot() {
    let entity = scriberig_core::EntityId::from_bytes([157; 16]);
    let snapshot = scriberig_core::SnapshotId::new([158; 32]);
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: Some(snapshot),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/explorer".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert_eq!(state.primary_view, crate::selection::PrimaryView::Explorer);
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Explorer);
    assert!(matches!(effects.as_slice(), [Effect::ListExplorer {
        entity: actual_entity,
        snapshot: actual_snapshot,
        continuation: None,
        ..
    }] if *actual_entity == entity && *actual_snapshot == snapshot));
    let escape = crate::event::Key {
        code: crate::event::KeyCode::Esc,
        modifiers: crate::event::Modifiers::default(),
    };
    let action = crate::keymap::action_for(&state, &escape).expect("view escape action");
    reduce(&mut state, action);
    assert_eq!(
        state.primary_view,
        crate::selection::PrimaryView::Conversation
    );
}

/// Opens the existing two-snapshot chooser from the `/diff` command.
#[test]
fn diff_slash_command_opens_snapshot_selection() {
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity: scriberig_core::EntityId::from_bytes([159; 16]),
            revision: None,
            snapshot: Some(scriberig_core::SnapshotId::new([160; 32])),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/diff".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(effects.as_slice(), [Effect::None]));
    assert!(
        matches!(state.overlay.as_ref(), Some(crate::overlay::Overlay::Input(input))
        if input.purpose == crate::overlay::input::InputPurpose::DiffSnapshot)
    );
    reduce(&mut state, Action::OverlayClosed);
    assert!(state.overlay.is_none());
    assert_eq!(
        state.primary_view,
        crate::selection::PrimaryView::Conversation
    );
}

/// Opens and exits the remaining major semantic auxiliary surfaces by command.
#[test]
fn auxiliary_slash_views_open_and_return_to_conversation() {
    let entity = scriberig_core::EntityId::from_bytes([161; 16]);
    let snapshot = scriberig_core::SnapshotId::new([162; 32]);
    let state_for_selection = || TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: Some(snapshot),
        }),
        ..TuiState::default()
    };
    let commands = [
        ("/history", crate::selection::PrimaryView::History),
        ("/find parser", crate::selection::PrimaryView::Find),
        ("/projection", crate::selection::PrimaryView::Projection),
        ("/changes", crate::selection::PrimaryView::Changes),
        ("/operations", crate::selection::PrimaryView::Operations),
        ("/inspect", crate::selection::PrimaryView::Overview),
    ];
    for (command, expected_view) in commands {
        let mut state = state_for_selection();
        state.conversation_composer.draft = command.to_owned();

        let effects = reduce(&mut state, Action::InputSubmitted);

        assert_ne!(
            state.primary_view,
            crate::selection::PrimaryView::Conversation
        );
        assert_eq!(state.primary_view, expected_view, "{command}");
        assert!(!effects.is_empty(), "{command}");
        let escape = crate::event::Key {
            code: crate::event::KeyCode::Esc,
            modifiers: crate::event::Modifiers::default(),
        };
        let action = crate::keymap::action_for(&state, &escape).expect("view escape action");
        reduce(&mut state, action);
        assert_eq!(
            state.primary_view,
            crate::selection::PrimaryView::Conversation
        );
    }
}

/// Verifies `/changes` refreshes the selected durable session before review.
#[test]
fn changes_command_refreshes_session_before_candidate_review() {
    let session = scriberig_core::EntityId::from_bytes([163; 16]);
    let operation =
        scriberig_core::OperationId::new("test-changes-refresh").expect("operation identity");
    let mut state = TuiState {
        agent_handles: vec![crate::service::AgentHandle {
            operation,
            session,
            branch: None,
            workspace: None,
            failure: None,
        }],
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/changes".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert_eq!(state.primary_view, crate::selection::PrimaryView::Changes);
    assert!(matches!(
        effects.as_slice(),
        [Effect::ReadSession {
            session: requested,
            branch: None,
            ..
        }] if *requested == session
    ));
}

/// Opens semantic source editing from the conversation composer.
#[test]
fn edit_slash_command_opens_projection_edit_for_exact_selection() {
    let entity = scriberig_core::EntityId::from_bytes([181; 16]);
    let snapshot = scriberig_core::SnapshotId::new([182; 32]);
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: Some(snapshot),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/edit".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.primary_view,
        crate::selection::PrimaryView::Projection
    );
    assert!(matches!(
        state.overlay,
        Some(crate::overlay::Overlay::Input(
            crate::overlay::input::InputState {
                purpose: crate::overlay::input::InputPurpose::ProjectionEdit,
                ..
            }
        ))
    ));
}

/// Sends plan advancement through the shared lifecycle operation.
#[test]
fn plan_start_command_invokes_shared_lifecycle_operation() {
    let session_id = scriberig_core::EntityId::from_bytes([131; 16]);
    let branch_id = scriberig_core::EntityId::from_bytes([132; 16]);
    let plan_id = scriberig_core::EntityId::from_bytes([133; 16]);
    let mut plan = scriberig_session::Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "Build parser",
    )
    .expect("plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("planned");
    let mut state = TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: session_id,
            branches: vec![scriberig_session::Branch::new(branch_id, "Root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        conversation_composer: crate::state::ConversationComposerState {
            target: crate::state::ConversationTarget::Existing {
                session: session_id,
                reply_anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: branch_id,
                },
            },
            draft: "/plan start".to_owned(),
            pending_prompt: None,
            pending: false,
            error: None,
        },
        ..TuiState::default()
    };

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        effects.as_slice(),
        [Effect::PlanLifecycle { request, action: scriberig_app::PlanLifecycleAction::Start }]
            if request.session == session_id
                && request.plan == plan_id
                && request.expected_revision == 1
    ));
}

/// Shows selected plan status and durable result counts locally.
#[test]
fn plan_show_reports_durable_plan_summary_without_execution() {
    let session_id = scriberig_core::EntityId::from_bytes([151; 16]);
    let branch_id = scriberig_core::EntityId::from_bytes([152; 16]);
    let plan_id = scriberig_core::EntityId::from_bytes([153; 16]);
    let mut plan = scriberig_session::Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "Build parser",
    )
    .expect("plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("planned");
    let mut state = TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: session_id,
            branches: vec![scriberig_session::Branch::new(branch_id, "Root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/plan show".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(effects.as_slice(), [Effect::None]));
    assert!(state.shell_message.contains("Build parser"));
    assert!(state.shell_message.contains("0/0 results"));
}

#[path = "state_session_composer_tests_part1_part2_part3.rs"]
mod part3;
