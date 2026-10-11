//! Tests the module's deterministic behavior.

use super::{Focus, TuiState, reduce};
use crate::action::{Action, Direction};
use crate::effect::Effect;
use crate::event::{Key, KeyCode, Modifiers, TuiEvent};
use crate::overlay::{Overlay, palette::PaletteState};

/// Verifies resize events update only terminal presentation state.
#[test]
fn resize_updates_state() {
    let mut state = TuiState::default();
    state.apply_event(&TuiEvent::Resize {
        width: 72,
        height: 24,
    });
    assert_eq!(state.terminal_size, (72, 24));
    assert!(!state.should_quit);
}

/// Verifies quit input stops the runtime.
#[test]
fn quit_key_sets_quit_flag() {
    let mut state = TuiState::default();
    state.apply_event(&TuiEvent::Key(Key {
        code: KeyCode::Char('q'),
        modifiers: Modifiers::default(),
    }));
    assert!(state.should_quit);
}

/// Verifies modal input takes precedence over browse commands.
#[test]
fn input_consumes_printable_keys() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::FindOpened);
    state.apply_event(&TuiEvent::Key(Key {
        code: KeyCode::Char('q'),
        modifiers: Modifiers::default(),
    }));
    assert_eq!(state.input_text(), Some("q"));
    assert!(!state.should_quit);
}

/// Verifies focus cycles without changing semantic history.
#[test]
fn focus_cycles_independently_of_navigation() {
    let mut state = TuiState::default();
    let history = state.navigation.clone();
    reduce(&mut state, Action::FocusNext);
    assert_eq!(state.focus, Focus::Inspector);
    reduce(&mut state, Action::FocusNext);
    assert_eq!(state.focus, Focus::Explorer);
    reduce(&mut state, Action::FocusNext);
    assert_eq!(state.focus, Focus::Primary);
    reduce(&mut state, Action::FocusPrevious);
    assert_eq!(state.focus, Focus::Explorer);
    assert_eq!(state.navigation, history);
}

/// Verifies overlays own input and close without changing navigation.
#[test]
fn overlay_closes_without_navigation_change() {
    let mut state = TuiState::default();
    let history = state.navigation.clone();
    reduce(&mut state, Action::HelpOpened);
    reduce(&mut state, Action::OverlayClosed);
    assert!(state.overlay.is_none());
    assert_eq!(state.navigation, history);
}

/// Verifies paste is inserted as one text payload.
#[test]
fn paste_is_input_data() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::FindOpened);
    reduce(&mut state, Action::Paste("q\n?".into()));
    assert_eq!(state.input_text(), Some("q\n?"));
    assert!(!state.should_quit);
}

/// Verifies pasted text is retained by the command palette.
#[test]
fn palette_paste_updates_filter_text() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::PaletteOpened);
    reduce(&mut state, Action::Paste("REF".into()));
    assert_eq!(
        state.overlay,
        Some(Overlay::Palette(PaletteState {
            query: "REF".to_owned(),
            ..PaletteState::default()
        }))
    );
}

/// Verifies palette cursor movement and activation stay local to the overlay.
#[test]
fn palette_selection_moves_and_activates() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::PaletteOpened);
    reduce(&mut state, Action::CursorMoved(Direction::Next));
    assert_eq!(
        state.overlay.as_ref().and_then(|overlay| match overlay {
            Overlay::Palette(palette) => Some(palette.selected),
            _ => None,
        }),
        Some(1)
    );
    let effects = reduce(&mut state, Action::Activated);
    assert_eq!(effects, vec![Effect::None]);
    assert!(state.overlay.is_none());
}

/// Verifies refresh re-reads the selected entity through the service port.
#[test]
fn refresh_reissues_selected_inspection() {
    let mut state = TuiState::default();
    let entity = scriberig_core::EntityId::from_bytes([7; 16]);
    state.selection = Some(crate::selection::EntitySelection {
        entity,
        revision: None,
        snapshot: None,
    });
    let effects = reduce(&mut state, Action::RefreshRequested);
    assert!(
        matches!(effects.as_slice(), [Effect::InspectEntity { entity: selected, .. }] if selected == &entity)
    );
}

/// Verifies application lifecycle events request authoritative operation state.
#[test]
fn application_operation_event_requests_authoritative_refresh() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("running").expect("operation id");
    let effects = state.apply_event(&crate::event::TuiEvent::Application(
        scriberig_app::ApplicationEvent::OperationStarted {
            operation: operation.clone(),
        },
    ));
    assert_eq!(effects, vec![Effect::OperationStatus { operation }]);
}

/// Verifies closing the operations presentation cannot cancel application work.
#[test]
fn closing_overlay_does_not_cancel_operation() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("active").expect("operation id");
    let record =
        scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Query);
    reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &record,
        )),
    );
    assert!(state.operations[0].cancellable());
    assert_eq!(
        reduce(&mut state, Action::OverlayClosed),
        vec![Effect::None]
    );
    assert!(state.operations[0].cancellable());
}

/// Verifies resizing preserves semantic navigation history.
#[test]
fn resize_preserves_navigation() {
    let mut state = TuiState::default();
    reduce(
        &mut state,
        Action::ActivateLocation(crate::selection::NavigationLocation::default()),
    );
    let history = state.navigation.clone();
    reduce(
        &mut state,
        Action::TerminalResized {
            width: 20,
            height: 8,
        },
    );
    assert_eq!(state.navigation, history);
}

/// Verifies invalid go-to input remains local and emits no inspect effect.
#[test]
fn invalid_entity_input_stays_in_overlay() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::FindOpened);
    reduce(&mut state, Action::InputChanged("not-an-entity".to_owned()));
    let effects = reduce(&mut state, Action::InputSubmitted);
    assert_eq!(effects, vec![Effect::None]);
    assert!(matches!(state.overlay, Some(Overlay::Input(input)) if input.error.is_some()));
    assert!(matches!(
        state.inspector,
        crate::inspector::LoadState::NotRequested
    ));
}

/// Verifies a late result cannot overwrite a newer inspector request.
#[test]
fn stale_inspect_result_is_ignored() {
    let mut state = TuiState::default();
    let entity = scriberig_core::EntityId::from_bytes([4; 16]);
    reduce(
        &mut state,
        Action::InspectRequested {
            request_id: 1,
            entity,
            snapshot: None,
        },
    );
    reduce(
        &mut state,
        Action::InspectRequested {
            request_id: 2,
            entity,
            snapshot: None,
        },
    );
    let model = crate::inspector::InspectorModel::from_result(scriberig_app::InspectResult {
        entity,
        revision: None,
        snapshot: None,
        object_type: None,
        label: Some("old".to_owned()),
        description: None,
        capabilities: Vec::new(),
        trust: None,
    });
    reduce(
        &mut state,
        Action::InspectSucceeded {
            request_id: 1,
            model,
        },
    );
    assert!(matches!(
        state.inspector,
        crate::inspector::LoadState::Loading { request_id: 2 }
    ));
}

/// Verifies switching primary views retains semantic selection.
#[test]
fn view_switch_preserves_selection() {
    let mut state = TuiState::default();
    let entity = scriberig_core::EntityId::from_bytes([8; 16]);
    state.selection = Some(crate::selection::EntitySelection {
        entity,
        revision: None,
        snapshot: None,
    });
    reduce(
        &mut state,
        Action::ViewSelected(crate::selection::PrimaryView::History),
    );
    assert_eq!(state.selection.map(|value| value.entity), Some(entity));
}

/// Verifies review confirmation is local until the confirmed command is emitted.
#[test]
fn confirmation_does_not_mutate_without_explicit_confirm() {
    let mut state = TuiState::default();
    let base = scriberig_core::SnapshotId::new([9; 32]);
    state.review = crate::inspector::LoadState::Ready(crate::view::changes::ChangeReviewModel {
        base_snapshot: base,
        change: "change-1".to_owned(),
        branch: None,
        affected_entities: Vec::new(),
        conflicts: Vec::new(),
        validation: crate::view::changes::ReviewValidation::Passed,
        requires_confirmation: true,
    });
    assert_eq!(
        reduce(&mut state, Action::ConfirmReview),
        vec![Effect::None]
    );
    assert!(matches!(state.overlay, Some(Overlay::Confirm(_))));
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Ready(_)
    ));
}

/// Verifies the confirmed commit carries the reviewed base snapshot.
#[test]
fn confirmed_commit_preserves_reviewed_base() {
    let mut state = TuiState::default();
    let base = scriberig_core::SnapshotId::new([6; 32]);
    let effects = reduce(
        &mut state,
        Action::CommitConfirmed(crate::view::changes::ConfirmedCommit {
            base_snapshot: base,
            change: "change-2".to_owned(),
            branch: None,
        }),
    );
    assert!(
        matches!(effects.as_slice(), [Effect::Commit { request_id: 1, request }]
            if request.base_snapshot == base && request.change == "change-2")
    );
}

/// Verifies a stale operation failure remains visible and is not retried.
#[test]
fn stale_operation_failure_is_visible_without_retry() {
    let mut state = TuiState::default();
    reduce(
        &mut state,
        Action::ReviewRequested {
            request: scriberig_app::ApplyRequest {
                base_snapshot: scriberig_core::SnapshotId::new([1; 32]),
                branch: None,
                change: "change".to_owned(),
            },
        },
    );
    let effects = reduce(
        &mut state,
        Action::OperationFailed {
            request_id: 1,
            category: "stale_base_snapshot".to_owned(),
            message: "base changed".to_owned(),
        },
    );
    assert_eq!(effects, vec![Effect::None]);
    assert!(
        matches!(state.review, crate::inspector::LoadState::Failed { ref category, .. } if category == "stale_base_snapshot")
    );
}

/// Verifies pasted multiline composer content remains data and submits intact.
#[test]
fn composer_submission_preserves_multiline_prompt() {
    let mut state = TuiState::default();
    let session = scriberig_core::EntityId::from_bytes([3; 16]);
    reduce(
        &mut state,
        Action::ComposerOpened {
            session,
            branch: None,
        },
    );
    reduce(&mut state, Action::Paste("line one\nline two".to_owned()));
    let effects = reduce(&mut state, Action::InputSubmitted);
    assert_eq!(
        effects,
        vec![Effect::SubmitAgent {
            session,
            branch: None,
            prompt: "line one\nline two".to_owned(),
        }]
    );
}

/// Verifies the submitted operation identity is tracked without replacement.
#[test]
fn submitted_agent_operation_id_is_stable() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("agent-1").expect("operation id");
    let effects = reduce(&mut state, Action::AgentSubmitted(operation.clone()));
    assert_eq!(
        effects,
        vec![Effect::OperationStatus {
            operation: operation.clone()
        }]
    );
    assert_eq!(
        effects,
        reduce(&mut state, Action::AgentSubmitted(operation))
    );
}

/// Verifies an authoritative progress record reaches the operation view.
#[test]
fn operation_progress_updates_presentation() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("progress").expect("operation id");
    let mut record =
        scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Query);
    record.status = scriberig_app::OperationStatus::Running;
    record.progress = Some(scriberig_app::OperationProgress {
        status: scriberig_app::OperationStatus::Running,
        completed: Some(3),
        total: Some(8),
        phase: None,
    });
    reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &record,
        )),
    );
    assert_eq!(state.operations[0].progress.as_deref(), Some("3/8"));
    assert_eq!(
        state.operations[0].status,
        scriberig_app::OperationStatus::Running
    );
}

#[path = "state_tail_tests.rs"]
mod state_tail_tests;
