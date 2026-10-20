//! Verifies presentation actions do not request model or tool work.

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

/// Verifies shared preferences fill untouched controls and preserve local choices.
#[test]
fn shared_preferences_resolve_beneath_local_tui_overrides() {
    let mut state = TuiState::default();
    state.set_presentation_override(crate::state::PresentationControl::ToolOutput);
    state.tool_output_visibility = crate::state::ToolOutputVisibility::Hidden;
    let effects = crate::state::reduce(
        &mut state,
        crate::action::Action::PresentationPreferencesLoaded(
            scriberig_protocol::AgentPresentationPreferencesResponse {
                revision: 4,
                preferences: scriberig_protocol::PublicAgentPresentationPreferences {
                    reasoning: scriberig_protocol::PublicReasoningVisibility::Shown,
                    tool_invocation: scriberig_protocol::PublicToolInvocationDetail::Detailed,
                    tool_output: scriberig_protocol::PublicToolOutputVisibility::Full,
                    prompt: scriberig_protocol::PublicPromptVisibility::Shown,
                },
            },
        ),
    );

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(state.presentation_preferences_revision, Some(4));
    assert_eq!(
        state.tool_output_visibility,
        crate::state::ToolOutputVisibility::Hidden
    );
    assert_eq!(
        state.tool_invocation_visibility,
        crate::state::ToolInvocationVisibility::StructuredParameters
    );
    assert_eq!(
        state.prompt_visibility,
        crate::state::PromptVisibility::Shown
    );
    assert_eq!(
        state.reasoning_visibility,
        crate::state::ReasoningVisibility::Shown
    );
}

/// Verifies full verbosity requests bounded safe detail without execution.
#[test]
fn tool_output_cycle_requests_only_missing_result_detail() {
    let session = scriberig_core::EntityId::from_bytes([71; 16]);
    let branch = scriberig_core::EntityId::from_bytes([72; 16]);
    let result = scriberig_core::EntityId::from_bytes([73; 16]);
    let mut state = TuiState {
        session: LoadState::Ready(crate::view::session::SessionModel {
            rows: vec![
                crate::view::session::SessionRow {
                    reference: session,
                    depth: 0,
                    label: "Session".to_owned(),
                    kind: crate::view::session::SessionRowKind::Session,
                    candidate: None,
                    snapshot: None,
                    transient: false,
                },
                crate::view::session::SessionRow {
                    reference: branch,
                    depth: 1,
                    label: "main".to_owned(),
                    kind: crate::view::session::SessionRowKind::Branch,
                    candidate: None,
                    snapshot: None,
                    transient: true,
                },
                crate::view::session::SessionRow {
                    reference: result,
                    depth: 3,
                    label: "Tool result: summary".to_owned(),
                    kind: crate::view::session::SessionRowKind::ToolResult,
                    candidate: None,
                    snapshot: None,
                    transient: true,
                },
            ],
            execution: scriberig_session::SessionExecutionSelection::default(),
            tool_invocation_details: std::collections::BTreeMap::default(),
        }),
        overlay: Some(crate::overlay::Overlay::Palette(
            crate::overlay::palette::PaletteState {
                query: String::new(),
                local_actions: vec![crate::overlay::palette::PaletteItem {
                    id: "toggle-tool-output".to_owned(),
                    label: "Toggle tool result summaries".to_owned(),
                    source: crate::overlay::palette::ActionSource::Local,
                }],
                semantic_actions: Vec::new(),
                selected: 0,
            },
        )),
        ..TuiState::default()
    };

    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);

    assert_eq!(
        effects,
        vec![Effect::ReadToolResultDetail {
            session,
            branch,
            result,
        }]
    );
    assert_eq!(
        state.tool_output_visibility,
        crate::state::ToolOutputVisibility::Full
    );
    assert!(state.shell_message.contains("Loading"));

    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);
    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.tool_output_visibility,
        crate::state::ToolOutputVisibility::Hidden
    );

    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.tool_output_visibility,
        crate::state::ToolOutputVisibility::Summary
    );
}

/// Verifies tool-call detail toggles locally without requesting execution.
#[test]
fn tool_invocation_detail_toggle_is_presentation_only() {
    let mut state = TuiState {
        overlay: Some(crate::overlay::Overlay::Palette(
            crate::overlay::palette::PaletteState {
                query: String::new(),
                local_actions: vec![crate::overlay::palette::PaletteItem {
                    id: "toggle-tool-invocation".to_owned(),
                    label: "Toggle tool call detail".to_owned(),
                    source: crate::overlay::palette::ActionSource::Local,
                }],
                semantic_actions: Vec::new(),
                selected: 0,
            },
        )),
        ..TuiState::default()
    };

    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.tool_invocation_visibility,
        crate::state::ToolInvocationVisibility::Full
    );
}

/// Verifies tool rendering mode cannot alter compiled context or request identity.
#[test]
fn tool_verbosity_changes_leave_context_fingerprint_unchanged() {
    let context = scriberig_agent_runtime::context::BranchExecutionContext::new(
        "session",
        "branch",
        "implement work",
        "continue local task",
    );
    let compiled = scriberig_agent_runtime::context::ContextCompiler::new(
        scriberig_agent_runtime::context::ContextPolicy::default(),
    )
    .compile(&context);
    let canonical = compiled.canonical.clone();
    let fingerprint = scriberig_agent_runtime::context::RequestFingerprintInput::new(
        "profile",
        "model",
        compiled.canonical,
        "tools",
    )
    .fingerprint();
    let mut state = TuiState::default();

    for _ in 0..4 {
        let effects = crate::state::activate_palette_control(&mut state, "toggle-tool-invocation")
            .expect("tool-call presentation control");
        assert_eq!(effects, vec![Effect::None]);
        let repeated = scriberig_agent_runtime::context::ContextCompiler::new(
            scriberig_agent_runtime::context::ContextPolicy::default(),
        )
        .compile(&context);
        let repeated_fingerprint = scriberig_agent_runtime::context::RequestFingerprintInput::new(
            "profile",
            "model",
            repeated.canonical.clone(),
            "tools",
        )
        .fingerprint();
        assert_eq!(repeated.canonical, canonical);
        assert_eq!(repeated_fingerprint, fingerprint);
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

/// Verifies navigation and presentation changes leave shared operations running.
#[test]
fn navigation_and_presentation_changes_do_not_cancel_operations() {
    let operation = scriberig_core::OperationId::new("active-agent").expect("operation id");
    let mut running =
        scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Agent);
    running.status = scriberig_app::OperationStatus::Running;
    let mut state = TuiState::default();
    crate::state::reduce(
        &mut state,
        crate::action::Action::OperationObserved(
            crate::view::operations::OperationModel::from_record(&running),
        ),
    );
    state.overlay = Some(crate::overlay::Overlay::Palette(
        crate::overlay::palette::PaletteState {
            query: String::new(),
            local_actions: vec![crate::overlay::palette::PaletteItem {
                id: "toggle-tool-invocation".to_owned(),
                label: "Toggle tool call detail".to_owned(),
                source: crate::overlay::palette::ActionSource::Local,
            }],
            semantic_actions: Vec::new(),
            selected: 0,
        },
    ));

    let preference_effects = crate::state::reduce(&mut state, crate::action::Action::Activated);
    let navigation_effects = crate::state::reduce(
        &mut state,
        crate::action::Action::ActivateLocation(crate::selection::NavigationLocation::default()),
    );
    let close_effects = crate::state::reduce(&mut state, crate::action::Action::OverlayClosed);

    for effects in [preference_effects, navigation_effects, close_effects] {
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, Effect::CancelOperation { .. }))
        );
    }
    assert!(state.operations[0].cancellable());
}

#[path = "state_presentation_tests/prompt.rs"]
mod prompt;
