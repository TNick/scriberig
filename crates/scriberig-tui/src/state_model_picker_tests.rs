//! Verifies the TUI model picker emits shared revision-bound mutations.

use super::*;

/// Creates a picker state with one loaded session and selected model.
///
/// # Parameters
/// - `model`: Optional current model identity.
///
/// # Returns
/// Deterministic state for model-control reducer tests.
fn picker_state(model: Option<&str>) -> TuiState {
    let session_id = EntityId::from_bytes([41; 16]);
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
            execution: scriberig_session::SessionExecutionSelection {
                revision: 12,
                model_profile: None,
                model: model.map(str::to_owned),
                thinking_level: None,
                history: Vec::new(),
            },
            tool_invocation_details: std::collections::BTreeMap::default(),
        }),
        model_library: LoadState::Ready(model_library()),
        ..TuiState::default()
    }
}

/// Creates two models with separate profiles and thinking levels.
///
/// # Returns
/// Deterministic protocol model descriptors for picker tests.
fn model_library() -> scriberig_protocol::ModelLibraryResponse {
    scriberig_protocol::ModelLibraryResponse {
        models: vec![
            scriberig_protocol::PublicModelDescriptor {
                id: "model-a".to_owned(),
                display_name: "Model A".to_owned(),
                provider_profile: "provider-a".to_owned(),
                provider_model_id: "provider-model-a".to_owned(),
                capabilities: scriberig_protocol::PublicModelCapabilities::default(),
                thinking_levels: vec![scriberig_protocol::PublicThinkingLevelDescriptor {
                    id: "careful".to_owned(),
                    label: "Careful".to_owned(),
                    description: None,
                    order: 1,
                }],
                default_thinking_level: Some("careful".to_owned()),
            },
            scriberig_protocol::PublicModelDescriptor {
                id: "model-b".to_owned(),
                display_name: "Model B".to_owned(),
                provider_profile: "provider-b".to_owned(),
                provider_model_id: "provider-model-b".to_owned(),
                capabilities: scriberig_protocol::PublicModelCapabilities::default(),
                thinking_levels: vec![scriberig_protocol::PublicThinkingLevelDescriptor {
                    id: "quick".to_owned(),
                    label: "Quick".to_owned(),
                    description: None,
                    order: 1,
                }],
                default_thinking_level: Some("quick".to_owned()),
            },
        ],
        profiles: vec![
            scriberig_protocol::PublicModelProfile {
                id: "profile-a".to_owned(),
                display_name: "Profile A".to_owned(),
                model: "model-a".to_owned(),
                connection: "connection-a".to_owned(),
                credential: None,
                default_thinking_level: Some("careful".to_owned()),
                enabled: true,
            },
            scriberig_protocol::PublicModelProfile {
                id: "profile-b".to_owned(),
                display_name: "Profile B".to_owned(),
                model: "model-b".to_owned(),
                connection: "connection-b".to_owned(),
                credential: None,
                default_thinking_level: Some("quick".to_owned()),
                enabled: true,
            },
        ],
    }
}

/// Verifies profile selection carries the current session revision.
#[test]
fn model_profile_selection_uses_shared_session_revision() {
    let mut state = picker_state(None);
    let effects = select_model_profile(&mut state, "profile-a");
    let session = EntityId::from_bytes([41; 16]);

    assert_eq!(
        effects,
        vec![Effect::SetSessionModel(
            scriberig_app::SessionModelSelectionRequest {
                session,
                expected_revision: 12,
                model_profile: "profile-a".to_owned(),
            }
        )]
    );
}

/// Verifies thinking selection rejects levels unsupported by the active model.
#[test]
fn unsupported_thinking_level_does_not_mutate_session() {
    let mut state = picker_state(Some("model-a"));

    let effects = select_thinking_level(&mut state, "private-level");

    assert_eq!(effects, vec![Effect::None]);
    assert!(state.shell_message.contains("not advertised"));
}

/// Verifies supported thinking selection carries the latest mutation revision.
#[test]
fn supported_thinking_selection_uses_shared_session_revision() {
    let mut state = picker_state(Some("model-a"));
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
    let mut state = picker_state(Some("model-old"));
    if let LoadState::Ready(session) = &mut state.session {
        session.execution.thinking_level = Some("deep".to_owned());
    }

    crate::state::reduce(
        &mut state,
        crate::action::Action::SessionExecutionSelectionUpdated(
            scriberig_session::SessionExecutionSelection {
                revision: 13,
                model_profile: Some("profile-a".to_owned()),
                model: Some("model-a".to_owned()),
                thinking_level: Some("careful".to_owned()),
                history: Vec::new(),
            },
        ),
    );

    let LoadState::Ready(session) = state.session else {
        panic!("session remains loaded");
    };
    assert_eq!(session.execution.revision, 13);
    assert!(session.rows[0].label.contains("Model model-a"));
    assert!(
        state
            .shell_message
            .contains("Model changed to model-a; thinking level adjusted to careful")
    );
}

/// Verifies the palette opens, loads profiles, and emits a shared mutation.
#[test]
fn palette_model_picker_loads_profiles_before_selecting_one() {
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
            .any(|item| item.id == "select-model:profile-b")
    );
    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);

    assert!(matches!(
        effects.as_slice(),
        [Effect::SetSessionModel(request)]
            if request.expected_revision == 12 && request.model_profile == "profile-a"
    ));
}

/// Verifies thinking controls load model-specific levels before presenting them.
#[test]
fn thinking_picker_loads_capabilities_then_emits_selection() {
    let mut state = picker_state(Some("model-a"));
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
    let mut state = picker_state(Some("model-b"));

    let effects = super::show_thinking_picker(&mut state);

    assert_eq!(effects, vec![Effect::None]);
    let Some(Overlay::Palette(palette)) = &state.overlay else {
        panic!("thinking picker opens for the selected model");
    };
    assert_eq!(palette.local_actions.len(), 1);
    assert_eq!(palette.local_actions[0].id, "select-thinking:quick");
}
