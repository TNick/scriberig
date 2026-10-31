//! Verifies presentation actions do not request model or tool work.

use super::*;

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
                local_actions: vec![crate::overlay::palette::PaletteItem {
                    id: "toggle-tool-output".to_owned(),
                    label: "Toggle tool result summaries".to_owned(),
                    source: crate::overlay::palette::ActionSource::Local,
                }],
                semantic_actions: Vec::new(),
            },
        )),
        ..TuiState::default()
    };

    let effects = crate::state::activate_palette_item(&mut state, "toggle-tool-output");

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

    let effects = crate::state::state_presentation::activate_palette_control(
        &mut state,
        "toggle-tool-output",
    )
    .expect("tool output toggle");
    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.tool_output_visibility,
        crate::state::ToolOutputVisibility::Hidden
    );

    let effects = crate::state::state_presentation::activate_palette_control(
        &mut state,
        "toggle-tool-output",
    )
    .expect("tool output toggle");

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
                local_actions: vec![crate::overlay::palette::PaletteItem {
                    id: "toggle-tool-invocation".to_owned(),
                    label: "Toggle tool call detail".to_owned(),
                    source: crate::overlay::palette::ActionSource::Local,
                }],
                semantic_actions: Vec::new(),
            },
        )),
        ..TuiState::default()
    };

    let effects = crate::state::activate_palette_item(&mut state, "toggle-tool-invocation");

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.tool_invocation_visibility,
        crate::state::ToolInvocationVisibility::Full
    );
}

/// Verifies local visibility controls preserve exact semantic and session state.
#[test]
fn presentation_toggles_leave_project_snapshot_and_session_unchanged() {
    let session = scriberig_core::EntityId::from_bytes([191; 16]);
    let snapshot = scriberig_core::SnapshotId::new([192; 32]);
    let session_snapshot = scriberig_session::Session {
        id: session,
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection {
            revision: 9,
            ..Default::default()
        },
    };
    let mut state = TuiState {
        project: Some("project-exact".to_owned()),
        geo_snapshot: Some(snapshot),
        session_snapshot: Some(session_snapshot.clone()),
        prompt_visibility: crate::state::PromptVisibility::Shown,
        ..TuiState::default()
    };

    for control in [
        "toggle-reasoning",
        "toggle-prompt",
        "toggle-tool-invocation",
    ] {
        assert_eq!(
            crate::state::activate_palette_control(&mut state, control),
            Some(vec![Effect::None])
        );
        assert_eq!(state.project.as_deref(), Some("project-exact"));
        assert_eq!(state.geo_snapshot, Some(snapshot));
        assert_eq!(state.session_snapshot.as_ref(), Some(&session_snapshot));
    }
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
            local_actions: vec![crate::overlay::palette::PaletteItem {
                id: "toggle-tool-invocation".to_owned(),
                label: "Toggle tool call detail".to_owned(),
                source: crate::overlay::palette::ActionSource::Local,
            }],
            semantic_actions: Vec::new(),
        },
    ));

    let preference_effects =
        crate::state::activate_palette_item(&mut state, "toggle-tool-invocation");
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

#[path = "state_presentation_tests/navigation.rs"]
mod navigation;
#[path = "state_presentation_tests/prompt.rs"]
mod prompt;

#[path = "state_presentation_tests/assignment.rs"]
mod assignment;
