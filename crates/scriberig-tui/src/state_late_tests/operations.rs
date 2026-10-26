//! Covers one focused group of TUI regression tests.

use super::*;

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
                candidate_snapshot: None,
                change: "candidate".to_owned(),
                branch: None,
                affected_entities: Vec::new(),
                deltas: Vec::new(),
                details: Vec::new(),
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
        Action::AgentOutputObserved {
            operation: scriberig_core::OperationId::new("previous-operation").expect("operation"),
            text: "previous answer".to_owned(),
            complete: false,
        },
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
    assert_eq!(state.transient_agent_outputs.len(), 0);
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
            failure: None,
        }),
    );
    assert!(
        matches!(effects.as_slice(), [Effect::OperationStatus { operation: observed }, Effect::ReadSession { session: actual, .. }] if observed == &operation && actual == &session)
    );
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Session);
    assert_eq!(state.agent_handles[0].operation, operation);
}
