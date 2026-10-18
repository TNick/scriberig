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
        crate::state::ToolInvocationVisibility::Detailed
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
        crate::state::ToolInvocationVisibility::Detailed
    );
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

/// Verifies prompt visibility loads captured requests without execution effects.
#[test]
fn prompt_visibility_requests_missing_model_details() {
    let session = scriberig_core::EntityId::from_bytes([81; 16]);
    let branch = scriberig_core::EntityId::from_bytes([82; 16]);
    let invocation = scriberig_core::EntityId::from_bytes([83; 16]);
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
                    reference: invocation,
                    depth: 2,
                    label: "Model: model-a".to_owned(),
                    kind: crate::view::session::SessionRowKind::ModelInvocation,
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
                    id: "toggle-prompt".to_owned(),
                    label: "Toggle captured model request".to_owned(),
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
        vec![Effect::ReadModelRequestDetail {
            session,
            branch,
            invocation,
        }]
    );
    assert_eq!(
        state.prompt_visibility,
        crate::state::PromptVisibility::Shown
    );

    let effects = crate::state::reduce(&mut state, crate::action::Action::Activated);

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.prompt_visibility,
        crate::state::PromptVisibility::Hidden
    );
}
