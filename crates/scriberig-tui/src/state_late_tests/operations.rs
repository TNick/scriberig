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

/// Candidate handle shared by the session projection and its review response.
const CANDIDATE: &str = "cnd_review";

/// Session identity used by the overlapping candidate review requests.
///
/// # Returns
/// The stable session identity shared by the request generations.
fn session_identity() -> scriberig_core::EntityId {
    scriberig_core::EntityId::from_bytes([21; 16])
}

/// Builds the session projection returned by one session-read generation.
///
/// - `candidate_snapshot`: Exact base snapshot for the durable candidate row, or
///   `None` to return a projection without any reviewable candidate row.
///
/// # Returns
/// A session projection for the shared session identity.
fn session_projection(
    candidate_snapshot: Option<scriberig_core::SnapshotId>,
) -> crate::view::session::SessionModel {
    let branch = scriberig_core::EntityId::from_bytes([22; 16]);
    let mut rows = vec![crate::view::session::SessionRow {
        reference: branch,
        depth: 1,
        label: "implementation branch".to_owned(),
        kind: crate::view::session::SessionRowKind::Branch,
        candidate: None,
        snapshot: None,
        transient: false,
    }];
    if let Some(snapshot) = candidate_snapshot {
        rows.push(crate::view::session::SessionRow {
            reference: branch,
            depth: 2,
            label: "Review candidate cnd_review".to_owned(),
            kind: crate::view::session::SessionRowKind::Candidate,
            candidate: Some(CANDIDATE.to_owned()),
            snapshot: Some(snapshot),
            transient: false,
        });
    }
    crate::view::session::SessionModel {
        rows,
        execution: scriberig_session::SessionExecutionSelection::default(),
        tool_invocation_details: std::collections::BTreeMap::default(),
    }
}

/// Builds one candidate review response.
///
/// - `base_snapshot`: Exact base snapshot carried by the reviewed candidate.
///
/// # Returns
/// A passing review model for the shared candidate handle.
fn review_response(
    base_snapshot: scriberig_core::SnapshotId,
) -> crate::view::changes::ChangeReviewModel {
    crate::view::changes::ChangeReviewModel {
        base_snapshot,
        candidate_snapshot: None,
        change: CANDIDATE.to_owned(),
        branch: None,
        affected_entities: Vec::new(),
        deltas: Vec::new(),
        details: vec!["scope: entity=ent_review snapshot=snp_base".to_owned()],
        conflicts: Vec::new(),
        validation: crate::view::changes::ReviewValidation::Passed,
        validation_profile: Some("project-review".to_owned()),
        requires_confirmation: true,
    }
}

/// Prepares the state a finished agent run leaves when a candidate review starts.
///
/// # Returns
/// A state with one durable agent handle and a loaded session that has no candidate yet.
fn state_before_candidate() -> TuiState {
    TuiState {
        primary_view: crate::selection::PrimaryView::Conversation,
        session: crate::inspector::LoadState::Ready(session_projection(None)),
        agent_handles: vec![crate::service::AgentHandle {
            operation: scriberig_core::OperationId::new("candidate-implementation")
                .expect("operation identity"),
            session: session_identity(),
            branch: None,
            workspace: None,
            failure: None,
        }],
        ..TuiState::default()
    }
}

/// Verifies the candidate review request survives a competing session refresh.
///
/// The Review candidate command marks the session loading and reads it, while a
/// durable session event starts a second read that supersedes the first. The review
/// must still open from the newest generation and read that generation's candidate.
#[test]
fn candidate_review_opens_after_a_superseding_session_refresh() {
    let base = scriberig_core::SnapshotId::new([4; 32]);
    let mut state = state_before_candidate();

    let refresh = crate::state::activate_palette_item(&mut state, "review-candidate");
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Changes);
    assert!(matches!(
        state.session,
        crate::inspector::LoadState::Loading { request_id: 1 }
    ));
    assert!(matches!(
        refresh.as_slice(),
        [Effect::ReadSession { request_id: 1, session, branch: None }] if *session == session_identity()
    ));

    // A durable session event starts its own read before the first response lands.
    let competing = reduce(
        &mut state,
        Action::SessionRequested {
            session: session_identity(),
            branch: None,
        },
    );
    assert!(matches!(
        state.session,
        crate::inspector::LoadState::Loading { request_id: 2 }
    ));
    assert!(matches!(
        competing.as_slice(),
        [Effect::ReadSession { request_id: 2, .. }]
    ));

    // The superseded response stays rejected by the stale-request guard.
    let stale_effects = reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 1,
            model: session_projection(Some(base)),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: None,
        },
    );
    assert_eq!(stale_effects, vec![Effect::None]);
    assert!(matches!(
        state.session,
        crate::inspector::LoadState::Loading { request_id: 2 }
    ));
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::NotRequested
    ));

    // The newest session generation carries the durable candidate and opens its review.
    let review = reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 2,
            model: session_projection(Some(base)),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: None,
        },
    );
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Loading { request_id: 3 }
    ));
    assert!(matches!(
        review.as_slice(),
        [Effect::Apply { request_id: 3, request }] if request.change == CANDIDATE && request.base_snapshot == base
    ));

    let settled = reduce(
        &mut state,
        Action::ReviewSucceeded {
            request_id: 3,
            model: review_response(base),
        },
    );
    assert_eq!(settled, vec![Effect::None]);
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Ready(_)
    ));
}

/// Verifies an older review response cannot settle the newest review generation.
///
/// Two review generations can overlap when a session refresh reopens the candidate
/// before the first Apply response lands. Only the newest generation's response may
/// replace the loading notice.
#[test]
fn superseded_review_response_cannot_settle_the_newest_generation() {
    let base = scriberig_core::SnapshotId::new([4; 32]);
    let mut state = state_before_candidate();
    let first = crate::state::activate_palette_item(&mut state, "review-candidate");
    assert!(matches!(
        first.as_slice(),
        [Effect::ReadSession { request_id: 1, .. }]
    ));
    let opened = reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 1,
            model: session_projection(Some(base)),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: None,
        },
    );
    assert!(matches!(
        opened.as_slice(),
        [Effect::Apply { request_id: 2, .. }]
    ));

    // A durable session event reopens the review for a newer generation.
    reduce(
        &mut state,
        Action::SessionRequested {
            session: session_identity(),
            branch: None,
        },
    );
    let reopened = reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 3,
            model: session_projection(Some(base)),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: None,
        },
    );
    assert!(matches!(
        reopened.as_slice(),
        [Effect::Apply { request_id: 4, .. }]
    ));

    let stale_effects = reduce(
        &mut state,
        Action::ReviewSucceeded {
            request_id: 2,
            model: review_response(base),
        },
    );
    assert_eq!(stale_effects, vec![Effect::None]);
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Loading { request_id: 4 }
    ));

    reduce(
        &mut state,
        Action::ReviewSucceeded {
            request_id: 4,
            model: review_response(base),
        },
    );
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Ready(_)
    ));
}
