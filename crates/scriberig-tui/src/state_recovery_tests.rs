//! Verifies local recovery command routing and explicit revision inputs.

use super::*;

/// Routes terminal recovery inspection and explicit model mapping locally.
#[test]
fn recovery_commands_require_a_loaded_session_and_emit_exact_mapping_intent() {
    let session_id = scriberig_core::EntityId::from_bytes([129; 16]);
    let mut state = TuiState::default();
    state.conversation_composer.draft = "/recover".to_owned();
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::None]
    ));
    assert!(
        state
            .conversation_composer
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("Load a session")
    );

    state.session_snapshot = Some(scriberig_session::Session {
        id: session_id,
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection {
            revision: 3,
            ..Default::default()
        },
    });
    state.conversation_composer.error = None;
    state.conversation_composer.draft = "/recover".to_owned();
    assert_eq!(
        reduce(&mut state, Action::InputSubmitted),
        vec![Effect::LegacyRecoveryList {
            session: session_id,
            selection_revision: 3,
            plan_revisions: Vec::new(),
        }]
    );

    state.conversation_composer.draft =
        "/recover map /execution 3 provider-a model-a high".to_owned();
    assert_eq!(
        reduce(&mut state, Action::InputSubmitted),
        vec![Effect::LegacyRecoveryMap {
            session: session_id,
            path: "/execution".to_owned(),
            expected_revision: 3,
            provider_id: "provider-a".to_owned(),
            model_id: "model-a".to_owned(),
            thinking_level: Some("high".to_owned()),
        }]
    );
}
