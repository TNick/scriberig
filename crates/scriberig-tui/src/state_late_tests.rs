//! Covers late TUI reducer interaction tests.

use super::*;
use crate::state::ConnectionState;

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

/// Verifies post-onboarding continuation uses the durable session handle.
#[test]
fn palette_continues_agent_session_after_workspace_selection() {
    let session = scriberig_core::EntityId::new();
    let semantic_root = scriberig_core::EntityId::new();
    let branch = scriberig_core::EntityId::new();
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity: semantic_root,
            revision: None,
            snapshot: Some(scriberig_core::SnapshotId::new([9; 32])),
        }),
        primary_view: crate::selection::PrimaryView::Session,
        session: crate::inspector::LoadState::Ready(crate::view::session::SessionModel {
            rows: vec![crate::view::session::SessionRow {
                reference: branch,
                depth: 1,
                label: "onboarding branch".to_owned(),
                kind: crate::view::session::SessionRowKind::Branch,
                candidate: None,
                snapshot: None,
                transient: false,
            }],
            execution: scriberig_session::SessionExecutionSelection::default(),
            tool_invocation_details: std::collections::BTreeMap::default(),
        }),
        agent_handles: vec![crate::service::AgentHandle {
            operation: scriberig_core::OperationId::new("test-operation")
                .expect("operation identity"),
            session,
            branch: Some(branch),
            workspace: None,
        }],
        ..TuiState::default()
    };
    reduce(&mut state, Action::PaletteOpened);
    reduce(
        &mut state,
        Action::InputChanged("continue-agent".to_owned()),
    );
    reduce(&mut state, Action::Activated);
    assert!(matches!(
        state.session_composer,
        Some(crate::state::SessionComposerState {
            session: selected_session,
            branch: Some(selected_branch),
            ..
        }) if selected_session == session && selected_branch == branch
    ));
    assert!(state.overlay.is_none());
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
        validation_profile: None,
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

/// Verifies successful commit advances the active selection to its successor snapshot.
#[test]
fn committed_candidate_refreshes_active_snapshot() {
    let mut state = TuiState::default();
    let entity = scriberig_core::EntityId::new();
    let base = scriberig_core::SnapshotId::new([5; 32]);
    let successor = scriberig_core::SnapshotId::new([6; 32]);
    state.project = Some(format!("project @ {base}"));
    state.selection = Some(crate::selection::EntitySelection {
        entity,
        revision: None,
        snapshot: Some(base),
    });
    state.review = crate::inspector::LoadState::Loading { request_id: 1 };
    let effects = reduce(
        &mut state,
        Action::CommitSucceeded {
            request_id: 1,
            model: crate::view::changes::ChangeReviewModel {
                base_snapshot: base,
                change: "candidate:1".to_owned(),
                branch: None,
                affected_entities: Vec::new(),
                conflicts: Vec::new(),
                validation: crate::view::changes::ReviewValidation::Passed,
                validation_profile: None,
                requires_confirmation: true,
            },
            snapshot: successor,
        },
    );
    assert_eq!(
        state.selection.and_then(|selection| selection.snapshot),
        Some(successor)
    );
    assert_eq!(state.project, Some(format!("project @ {successor}")));
    assert!(
        matches!(effects.as_slice(), [Effect::InspectEntity { entity: target, snapshot: Some(value), .. }] if *target == entity && *value == successor)
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

/// Verifies filesystem conflicts remain operation failures and clear after review succeeds.
#[test]
fn candidate_conflict_preserves_connection_and_clears_after_reinspection() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::CapabilitiesLoaded(Vec::new()));
    let request = scriberig_app::ApplyRequest {
        base_snapshot: scriberig_core::SnapshotId::new([4; 32]),
        branch: None,
        change: "candidate".to_owned(),
    };
    reduce(
        &mut state,
        Action::ReviewRequested {
            request: request.clone(),
        },
    );
    reduce(
        &mut state,
        Action::OperationFailed {
            request_id: 1,
            category: "filesystem_conflict".to_owned(),
            message: "source changed externally: Cargo.toml".to_owned(),
        },
    );
    assert_eq!(state.connection, ConnectionState::Connected);
    assert!(state.operation_error.is_some());
    reduce(&mut state, Action::ReviewRequested { request });
    reduce(
        &mut state,
        Action::ReviewSucceeded {
            request_id: 2,
            model: crate::view::changes::ChangeReviewModel {
                base_snapshot: scriberig_core::SnapshotId::new([4; 32]),
                change: "candidate".to_owned(),
                branch: None,
                affected_entities: Vec::new(),
                conflicts: vec!["update Cargo.toml".to_owned()],
                validation: crate::view::changes::ReviewValidation::Passed,
                validation_profile: None,
                requires_confirmation: true,
            },
        },
    );
    assert_eq!(state.connection, ConnectionState::Connected);
    assert_eq!(state.operation_error, None);
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Ready(_)
    ));
}

/// Verifies pasted multiline composer content remains data and submits intact.
#[test]
fn composer_submission_preserves_multiline_prompt() {
    let mut state = TuiState::default();
    let session = scriberig_core::EntityId::from_bytes([3; 16]);
    reduce(
        &mut state,
        Action::AgentOutputObserved("previous answer".to_owned()),
    );
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
    assert_eq!(state.agent_output_observations, Vec::<String>::new());
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
