//! Verifies persistent session-composer behavior and submission semantics.

use super::*;

/// Submits a persistent session draft through the existing shared agent effect.
#[test]
fn persistent_session_composer_submits_without_a_modal() {
    let mut state = TuiState::default();
    let session = scriberig_core::EntityId::from_bytes([31; 16]);
    let branch = scriberig_core::EntityId::from_bytes([32; 16]);
    let open_effects = reduce(
        &mut state,
        Action::SessionComposerOpened {
            session,
            branch: Some(branch),
        },
    );

    assert!(
        matches!(open_effects.as_slice(), [Effect::ReadSession { session: value, .. }] if *value == session)
    );
    assert!(state.overlay.is_none());
    assert!(state.is_input_active());
    reduce(&mut state, Action::InputChanged("inspect this".to_owned()));
    reduce(&mut state, Action::InputChanged("\n".to_owned()));
    reduce(&mut state, Action::InputChanged("entity".to_owned()));
    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(
        matches!(effects.as_slice(), [Effect::SubmitAgent { session: value, branch: Some(target), prompt }] if *value == session && *target == branch && prompt == "inspect this\nentity")
    );
    let composer = state
        .session_composer
        .as_ref()
        .expect("persistent composer");
    assert_eq!(
        composer.pending_prompt.as_deref(),
        Some("inspect this\nentity")
    );
    assert_eq!(composer.text, "");
    assert!(state.overlay.is_none());
    reduce(&mut state, Action::InputChanged("next prompt".to_owned()));
    assert_eq!(
        reduce(&mut state, Action::InputSubmitted),
        vec![Effect::None]
    );

    reduce(
        &mut state,
        Action::AgentSubmitted(crate::service::AgentHandle {
            operation: scriberig_core::OperationId::new("agent-local-31").expect("operation id"),
            session,
            branch: Some(branch),
            workspace: None,
            failure: None,
        }),
    );

    let composer = state
        .session_composer
        .as_ref()
        .expect("persistent composer");
    assert!(composer.pending_prompt.is_none());
    assert_eq!(composer.session, session);
    assert_eq!(composer.branch, Some(branch));
    assert_eq!(composer.text, "next prompt");

    let queued_effects = reduce(&mut state, Action::InputSubmitted);
    assert!(matches!(
        queued_effects.as_slice(),
        [Effect::SubmitAgent {
            session: value,
            branch: Some(target),
            prompt
        }] if *value == session && *target == branch && prompt == "next prompt"
    ));
    reduce(
        &mut state,
        Action::AgentSubmitted(crate::service::AgentHandle {
            operation: scriberig_core::OperationId::new("agent-local-32").expect("operation id"),
            session,
            branch: Some(branch),
            workspace: None,
            failure: None,
        }),
    );
    reduce(&mut state, Action::InputChanged("while running".to_owned()));
    let effects = reduce(&mut state, Action::InputSubmitted);
    assert!(matches!(
        effects.as_slice(),
        [Effect::SubmitAgent { prompt, .. }] if prompt == "while running"
    ));
    assert_eq!(state.session_composer.as_ref().unwrap().text, "");
}

/// Opens a new composer without reading a session that is not persisted yet.
#[test]
fn new_session_composer_does_not_read_missing_session() {
    let mut state = TuiState::default();
    let session = scriberig_core::EntityId::from_bytes([41; 16]);

    let effects = reduce(
        &mut state,
        Action::SessionComposerOpened {
            session,
            branch: None,
        },
    );

    assert_eq!(effects, vec![Effect::None]);
    assert!(matches!(
        state.session,
        crate::inspector::LoadState::NotRequested
    ));
}

/// Refreshes the active session when another client changes its selection.
#[test]
fn session_change_event_refreshes_the_active_composer_session() {
    let mut state = TuiState::default();
    let session = scriberig_core::EntityId::from_bytes([33; 16]);
    let branch = scriberig_core::EntityId::from_bytes([34; 16]);
    state.session_composer = Some(crate::state::SessionComposerState {
        session,
        branch: Some(branch),
        text: String::new(),
        pending_prompt: None,
        error: None,
    });

    let effects =
        state.apply_application_event(&scriberig_app::ApplicationEvent::SessionChanged { session });

    assert!(matches!(
        effects.as_slice(),
        [Effect::ReadSession {
            session: requested,
            branch: Some(requested_branch),
            ..
        }] if *requested == session && *requested_branch == branch
    ));
}

/// Keeps pasted composer content literal instead of replaying palette commands.
#[test]
fn persistent_session_composer_keeps_paste_as_text() {
    let mut state = TuiState::default();
    reduce(
        &mut state,
        Action::SessionComposerOpened {
            session: scriberig_core::EntityId::from_bytes([33; 16]),
            branch: None,
        },
    );
    reduce(&mut state, Action::Paste("start-agent\n/model".to_owned()));

    assert_eq!(
        state
            .session_composer
            .as_ref()
            .expect("persistent composer")
            .text,
        "start-agent\n/model"
    );
}

/// Lets the conversation home capture a true pre-session first message.
#[test]
fn conversation_home_types_and_submits_without_allocating_local_identity() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::InputChanged("h".to_owned()));
    reduce(&mut state, Action::InputChanged("ello".to_owned()));

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert_eq!(state.conversation_composer.draft, "");
    assert_eq!(
        state.conversation_composer.pending_prompt.as_deref(),
        Some("hello")
    );
    assert!(state.conversation_composer.pending);
    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::NewSession
    );
    assert!(matches!(
        effects.as_slice(),
        [Effect::StartNewConversation {
            prompt,
            selection: None
        }] if prompt == "hello"
    ));

    reduce(
        &mut state,
        Action::OperationFailed {
            request_id: 0,
            category: "unsupported_operation".to_owned(),
            message: "configure a provider connection".to_owned(),
        },
    );

    assert_eq!(state.conversation_composer.draft, "hello");
    assert!(state.conversation_composer.pending_prompt.is_none());
    assert!(!state.conversation_composer.pending);
    assert_eq!(
        state.conversation_composer.error.as_deref(),
        Some("configure a provider connection")
    );
}

/// Carries the exact configured semantic entity into a first agent session.
#[test]
fn first_conversation_inherits_exact_semantic_selection() {
    let project = scriberig_core::ProjectId::from_bytes([1; 16]);
    let root = scriberig_core::EntityId::from_bytes([2; 16]);
    let entity = scriberig_core::EntityId::from_bytes([3; 16]);
    let snapshot = scriberig_core::SnapshotId::new([4; 32]);
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root,
            entity,
            snapshot,
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "inspect source".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        effects.as_slice(),
        [Effect::StartNewConversation {
            selection: Some(selected),
            ..
        }] if selected.project == Some(project)
            && selected.root == root
            && selected.entity == entity
            && selected.snapshot == snapshot
    ));
}

/// Keeps unknown commands local and honors the explicit literal slash escape.
#[test]
fn conversation_slash_routing_never_sends_unknown_commands_to_the_agent() {
    let mut state = TuiState::default();
    state.conversation_composer.draft = "/not-a-command".to_owned();
    let unknown = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(unknown.as_slice(), [Effect::None]));
    assert!(
        state
            .conversation_composer
            .error
            .as_deref()
            .is_some_and(|message| message.contains("Unknown command"))
    );
    assert_eq!(state.conversation_composer.draft, "/not-a-command");

    state.conversation_composer.error = None;
    state.conversation_composer.draft = "//literal".to_owned();
    let literal = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        literal.as_slice(),
        [Effect::StartNewConversation {
            prompt,
            selection: None
        }] if prompt == "/literal"
    ));

    state.conversation_composer.pending = false;
    state.conversation_composer.draft = "/model".to_owned();
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::None]
    ));
    assert_eq!(state.conversation_composer.draft, "");
    let session = scriberig_core::EntityId::from_bytes([125; 16]);
    let branch = scriberig_core::EntityId::from_bytes([126; 16]);
    state.conversation_composer.target = crate::state::ConversationTarget::Existing {
        session,
        reply_anchor: scriberig_session::ContinuationAnchor::BranchHead { branch },
    };
    state.conversation_composer.draft = [
        "/plan improve parser",
        "- inspect: map existing parser behavior",
        "- design: choose a compatible grammar",
        "- implement: update parser and verify fixtures",
        "depends: 1 -> 3",
    ]
    .join("\n");
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::CreatePlan(request)]
            if request.goal == "improve parser"
                && request.session == session
                && request.anchor == scriberig_session::ContinuationAnchor::BranchHead { branch }
                && request.nodes.len() == 3
                && request.nodes[0].title == "inspect"
                && request.nodes[2].title == "implement"
                && request.dependencies == vec![(0, 2)]
    ));
}

/// Routes exact session-management commands to local session effects.
#[test]
fn conversation_session_commands_are_local_and_exact() {
    let id = scriberig_core::EntityId::from_bytes([127; 16]);
    let mut state = TuiState::default();

    state.conversation_composer.draft = "/sessions".to_owned();
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::ListSessions]
    ));

    state.conversation_composer.draft = format!("/switch {id}");
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::SwitchSession { session }] if *session == id
    ));

    state.conversation_composer.draft = format!("/remove-session {id}");
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::RemoveSession { session }] if *session == id
    ));

    state.conversation_composer.draft = "/new".to_owned();
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::None]
    ));
    assert_eq!(
        state.conversation_composer.target,
        crate::state::ConversationTarget::NewSession
    );
}

/// Routes a local auxiliary command while a provider turn is still running.
#[test]
fn conversation_auxiliary_command_is_available_while_agent_is_pending() {
    let mut state = TuiState::default();
    state.conversation_composer.draft = "request currently running".to_owned();

    reduce(&mut state, Action::InputSubmitted);
    assert_eq!(state.conversation_composer.draft, "");
    assert_eq!(
        state.conversation_composer.pending_prompt.as_deref(),
        Some("request currently running")
    );
    reduce(&mut state, Action::InputChanged("/operations".to_owned()));

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert_eq!(
        state.primary_view,
        crate::selection::PrimaryView::Operations
    );
    assert!(matches!(effects.as_slice(), [Effect::ListOperations]));
    assert!(state.conversation_composer.pending);
    assert_eq!(
        state.conversation_composer.pending_prompt.as_deref(),
        Some("request currently running")
    );
}

/// Verifies the project slash command reports a missing project.
#[test]
fn project_command_reports_missing_project() {
    let mut state = TuiState::default();
    state.conversation_composer.draft = "/project".to_owned();
    reduce(&mut state, Action::InputSubmitted);
    assert!(
        state
            .shell_message
            .contains("No active project is selected.")
    );
}

#[path = "state_session_composer_tests_part1.rs"]
mod part1;
