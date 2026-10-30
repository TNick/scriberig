//! Verifies presentation and plan-assignment picker behavior.

use super::*;

/// Creates a safe model metadata fixture for presentation actions.
///
/// # Returns
/// A neutral upstream metadata snapshot for model control fixtures.
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

/// Reports a missing session when a queued assignment picker is activated.
#[test]
fn plan_assignment_picker_reports_lost_session_context() {
    let model = scriberig_protocol::PublicModelDescriptor {
        id: "descriptor-a".to_owned(),
        display_name: "Model A".to_owned(),
        provider_id: "provider-a".to_owned(),
        provider_model_id: "model-a".to_owned(),
        thinking_levels: Vec::new(),
        metadata_snapshot: model_metadata_snapshot(),
    };
    let mut state = TuiState {
        plan_assignment_target: Some(crate::state::PlanAssignmentTarget {
            plan: scriberig_core::EntityId::from_bytes([181; 16]),
            node: scriberig_core::EntityId::from_bytes([182; 16]),
            expected_revision: 1,
        }),
        model_library: crate::inspector::LoadState::Ready(
            scriberig_protocol::ModelLibraryResponse {
                provider_readiness: Vec::new(),
                models: vec![model],
            },
        ),
        ..TuiState::default()
    };

    let effects =
        crate::state::activate_palette_control(&mut state, "assign-plan-model:descriptor-a")
            .expect("assignment action");

    assert_eq!(effects, vec![Effect::None]);
    assert!(
        state
            .conversation_composer
            .error
            .as_deref()
            .is_some_and(|message| message.contains("Load the plan session"))
    );
}

/// Resolves a displayed model selection to its upstream provider/model pair.
#[test]
fn plan_assignment_uses_selected_model_after_reordering() {
    let session = scriberig_core::EntityId::from_bytes([111; 16]);
    let node = scriberig_core::EntityId::from_bytes([112; 16]);
    let plan = scriberig_core::EntityId::from_bytes([113; 16]);
    let model =
        |id: &str, provider: &str, model_id: &str| scriberig_protocol::PublicModelDescriptor {
            id: id.to_owned(),
            display_name: id.to_owned(),
            provider_id: provider.to_owned(),
            provider_model_id: model_id.to_owned(),
            thinking_levels: Vec::new(),
            metadata_snapshot: model_metadata_snapshot(),
        };
    let mut state = TuiState {
        plan_assignment_target: Some(crate::state::PlanAssignmentTarget {
            plan,
            node,
            expected_revision: 7,
        }),
        session_snapshot: Some(scriberig_session::Session {
            id: session,
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        model_library: crate::inspector::LoadState::Ready(
            scriberig_protocol::ModelLibraryResponse {
                provider_readiness: Vec::new(),
                models: vec![
                    model("descriptor-b", "provider-b", "model-b"),
                    model("descriptor-a", "provider-a", "model-a"),
                ],
            },
        ),
        ..TuiState::default()
    };

    let effects =
        crate::state::activate_palette_control(&mut state, "assign-plan-model:descriptor-a")
            .expect("assignment action");

    assert!(matches!(
        effects.as_slice(),
        [Effect::SetPlanNodeAssignment(request)]
            if request.assignment.as_ref().is_some_and(|assignment| {
                assignment.provider_id == "provider-a"
                    && assignment.model_id == "model-a"
                    && assignment.thinking_level.is_none()
            })
    ));
}
