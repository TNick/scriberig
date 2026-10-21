//! Verifies the TUI model picker emits shared revision-bound mutations.

use super::*;

/// Creates a safe model metadata fixture for picker rows.
///
/// # Returns
/// A neutral upstream metadata snapshot for model picker fixtures.
fn model_metadata_snapshot() -> scriberig_protocol::PublicModelMetadataSnapshot {
    scriberig_protocol::PublicModelMetadataSnapshot {
        schema_version: "rpi-safe-model-snapshot-v1".to_owned(),
        api_id: "openai-completions".to_owned(),
        reasoning: false,
        input: vec![scriberig_protocol::PublicModelInputModality::Text],
        cost: scriberig_protocol::PublicModelCostSnapshot {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            tiers: Vec::new(),
        },
        cost_configured: false,
        context_window: 4096,
        max_tokens: 1024,
    }
}

/// Creates a picker state with one loaded session and selected model.
///
/// # Parameters
/// - `selection`: Optional current provider and model identity.
///
/// # Returns
/// A deterministic state for model-control reducer tests.
fn picker_state(selection: Option<(&str, &str)>) -> TuiState {
    let session_id = EntityId::from_bytes([41; 16]);
    let execution = scriberig_session::SessionExecutionSelection {
        revision: 12,
        provider_id: selection.map(|(provider, _)| provider.to_owned()),
        model_id: selection.map(|(_, model)| model.to_owned()),
        thinking_level: None,
        history: Vec::new(),
    };
    TuiState {
        session: LoadState::Ready(crate::view::session::SessionModel {
            rows: vec![crate::view::session::SessionRow {
                reference: session_id,
                depth: 0,
                label: "session".to_owned(),
                kind: crate::view::session::SessionRowKind::Session,
                candidate: None,
                snapshot: None,
                transient: false,
            }],
            execution: execution.clone(),
            tool_invocation_details: std::collections::BTreeMap::default(),
        }),
        session_snapshot: Some(scriberig_session::Session {
            id: session_id,
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution,
        }),
        model_library: LoadState::Ready(model_library()),
        ..TuiState::default()
    }
}

/// Creates two displayed upstream models with separate thinking levels.
///
/// # Returns
/// Two safe model rows with distinct provider and thinking-level identities.
fn model_library() -> scriberig_protocol::ModelLibraryResponse {
    scriberig_protocol::ModelLibraryResponse {
        provider_readiness: Vec::new(),
        models: vec![
            scriberig_protocol::PublicModelDescriptor {
                id: "model-a".to_owned(),
                display_name: "Model A".to_owned(),
                provider_id: "provider-a".to_owned(),
                provider_model_id: "provider-model-a".to_owned(),
                thinking_levels: vec!["careful".to_owned()],
                metadata_snapshot: model_metadata_snapshot(),
            },
            scriberig_protocol::PublicModelDescriptor {
                id: "model-b".to_owned(),
                display_name: "Model B".to_owned(),
                provider_id: "provider-b".to_owned(),
                provider_model_id: "provider-model-b".to_owned(),
                thinking_levels: vec!["quick".to_owned()],
                metadata_snapshot: model_metadata_snapshot(),
            },
        ],
    }
}

/// Verifies a displayed model selection carries the current session revision.
#[test]
fn displayed_model_selection_uses_shared_session_revision() {
    let mut state = picker_state(None);
    let effects = select_model(&mut state, "model-a");
    let session = EntityId::from_bytes([41; 16]);

    assert_eq!(
        effects,
        vec![Effect::SetSessionModel(
            scriberig_app::SessionModelSelectionRequest {
                session,
                expected_revision: 12,
                provider_id: "provider-a".to_owned(),
                model_id: "provider-model-a".to_owned(),
            }
        )]
    );
}

/// Verifies thinking selection rejects levels unsupported by the active model.
#[test]
fn unsupported_thinking_level_does_not_mutate_session() {
    let mut state = picker_state(Some(("provider-a", "provider-model-a")));

    let effects = select_thinking_level(&mut state, "private-level");

    assert_eq!(effects, vec![Effect::None]);
    assert!(state.shell_message.contains("not advertised"));
}

/// Verifies supported thinking selection carries the latest mutation revision.
#[test]
fn supported_thinking_selection_uses_shared_session_revision() {
    let mut state = picker_state(Some(("provider-a", "provider-model-a")));
    let session = EntityId::from_bytes([41; 16]);

    let effects = select_thinking_level(&mut state, "careful");

    assert_eq!(
        effects,
        vec![Effect::SetSessionThinkingLevel(
            scriberig_app::SessionThinkingSelectionRequest {
                session,
                expected_revision: 12,
                thinking_level: "careful".to_owned(),
            }
        )]
    );
}

/// Verifies mutation responses replace the displayed authoritative selection.
#[test]
fn selection_response_updates_session_header_and_revision() {
    let mut state = picker_state(Some(("provider-a", "model-old")));
    if let LoadState::Ready(session) = &mut state.session {
        session.execution.thinking_level = Some("deep".to_owned());
    }

    crate::state::reduce(
        &mut state,
        crate::action::Action::SessionExecutionSelectionUpdated(
            scriberig_session::SessionExecutionSelection {
                revision: 13,
                provider_id: Some("provider-a".to_owned()),
                model_id: Some("provider-model-a".to_owned()),
                thinking_level: Some("careful".to_owned()),
                history: Vec::new(),
            },
        ),
    );

    let LoadState::Ready(session) = state.session else {
        panic!("session remains loaded");
    };
    assert_eq!(session.execution.revision, 13);
    assert!(session.rows[0].label.contains("Model provider-model-a"));
    assert_eq!(
        state
            .session_snapshot
            .as_ref()
            .map(|session| session.execution.provider_id.as_deref()),
        Some(Some("provider-a"))
    );
    assert!(
        state
            .shell_message
            .contains("Model changed to provider-model-a; thinking level adjusted to careful")
    );
}

/// Verifies the palette opens, loads displayed models, and emits a shared mutation.
#[test]
fn palette_model_picker_loads_models_before_selecting_one() {
    let mut state = picker_state(None);
    state.model_library = LoadState::NotRequested;
    state.overlay = Some(Overlay::Palette(PaletteState {
        query: String::new(),
        local_actions: vec![crate::overlay::palette::PaletteItem {
            id: "model-picker".to_owned(),
            label: "Select model".to_owned(),
            source: crate::overlay::palette::ActionSource::Local,
        }],
        semantic_actions: Vec::new(),
        selected: 0,
    }));

    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);
    assert!(matches!(
        effects.as_slice(),
        [Effect::ReadModelLibrary { request_id: 1 }]
    ));

    crate::state::reduce(
        &mut state,
        crate::action::Action::ModelLibraryLoaded {
            request_id: 1,
            library: model_library(),
        },
    );
    let Some(Overlay::Palette(palette)) = &state.overlay else {
        panic!("model picker opens after the library loads");
    };
    assert_eq!(palette.local_actions.len(), 2);
    assert!(
        palette
            .local_actions
            .iter()
            .any(|item| item.id == "select-model:model-b")
    );
    assert!(
        palette
            .local_actions
            .iter()
            .any(|item| item.label == "Model B · provider-b/provider-model-b")
    );
    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);

    assert!(matches!(
        effects.as_slice(),
        [Effect::SetSessionModel(request)]
            if request.expected_revision == 12
                && request.provider_id == "provider-a"
                && request.model_id == "provider-model-a"
    ));
}

/// Verifies an empty model library reports a visible composer message.
#[test]
fn model_picker_reports_when_no_models_are_available() {
    let mut state = picker_state(None);
    let mut library = model_library();
    library.models.clear();

    show_model_picker(&mut state, &library);

    assert!(state.overlay.is_none());
    assert_eq!(
        state.conversation_composer.error.as_deref(),
        Some("No models are available.")
    );
}

/// Verifies thinking controls load model-specific levels before presenting them.
#[test]
fn thinking_picker_loads_capabilities_then_emits_selection() {
    let mut state = picker_state(Some(("provider-a", "provider-model-a")));
    state.model_library = LoadState::NotRequested;

    let effects = super::activate_model_picker_action(&mut state, "thinking-picker")
        .expect("thinking picker action");
    assert!(matches!(
        effects.as_slice(),
        [Effect::ReadModelLibrary { .. }]
    ));

    crate::state::reduce(
        &mut state,
        crate::action::Action::ModelLibraryLoaded {
            request_id: 1,
            library: model_library(),
        },
    );
    let Some(Overlay::Palette(palette)) = &state.overlay else {
        panic!("thinking picker opens after the library loads");
    };
    assert_eq!(palette.local_actions.len(), 1);
    assert_eq!(palette.local_actions[0].id, "select-thinking:careful");
    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);

    assert!(matches!(
        effects.as_slice(),
        [Effect::SetSessionThinkingLevel(request)]
            if request.expected_revision == 12 && request.thinking_level == "careful"
    ));
}

/// Verifies the thinking palette follows a changed shared model identity.
#[test]
fn thinking_picker_tracks_the_selected_models_advertised_levels() {
    let mut state = picker_state(Some(("provider-b", "provider-model-b")));

    let effects = super::show_thinking_picker(&mut state);

    assert_eq!(effects, vec![Effect::None]);
    let Some(Overlay::Palette(palette)) = &state.overlay else {
        panic!("thinking picker opens for the selected model");
    };
    assert_eq!(palette.local_actions.len(), 1);
    assert_eq!(palette.local_actions[0].id, "select-thinking:quick");
}
