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
    let newline = crate::keymap::action_for(
        &state,
        &crate::event::Key {
            code: crate::event::KeyCode::Char('j'),
            modifiers: crate::event::Modifiers {
                control: true,
                ..Default::default()
            },
        },
    )
    .expect("Ctrl+J newline binding");
    reduce(&mut state, newline);
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

    let effects = state.apply_event(&crate::event::TuiEvent::Application(
        scriberig_app::ApplicationEvent::SessionChanged { session },
    ));

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

/// Routes console slash commands to shared model and view reducers.
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
