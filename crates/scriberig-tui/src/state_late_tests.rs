//! Covers late TUI reducer interaction tests.

use super::*;

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

/// Verifies find input dispatches a snapshot-scoped discovery request.
#[test]
fn invalid_entity_input_stays_in_overlay() {
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity: scriberig_core::EntityId::from_bytes([1; 16]),
            revision: None,
            snapshot: Some(scriberig_core::SnapshotId::new([2; 32])),
        }),
        ..TuiState::default()
    };
    reduce(&mut state, Action::FindOpened);
    reduce(&mut state, Action::InputChanged("not-an-entity".to_owned()));
    let effects = reduce(&mut state, Action::InputSubmitted);
    assert_eq!(
        effects,
        vec![Effect::RequestFind {
            request_id: 1,
            text: "not-an-entity".to_owned(),
            snapshot: scriberig_core::SnapshotId::new([2; 32]),
        }]
    );
    assert!(state.overlay.is_none());
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
        matches!(effects.as_slice(), [Effect::Commit { request_id: 1, request }] if request.base_snapshot == base && request.change == "change-2")
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
    assert_eq!(
        reduce(&mut state, Action::InputSubmitted),
        vec![Effect::SubmitAgent {
            session,
            branch: None,
            prompt: "line one\nline two".to_owned(),
        }]
    );
}

/// Verifies submission refreshes both operation and session state.
#[test]
fn submitted_agent_operation_id_is_stable() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("agent-1").expect("operation id");
    let session = scriberig_core::EntityId::from_bytes([1; 16]);
    let effects = reduce(
        &mut state,
        Action::AgentSubmitted(crate::service::AgentHandle {
            operation: operation.clone(),
            session,
            branch: None,
            workspace: None,
        }),
    );
    assert!(
        matches!(effects.as_slice(), [Effect::OperationStatus { operation: observed }, Effect::ReadSession { session: actual, .. }] if observed == &operation && actual == &session)
    );
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Session);
    assert_eq!(state.agent_handles[0].operation, operation);
}
