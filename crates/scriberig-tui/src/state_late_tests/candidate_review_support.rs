//! Shared candidate, session, and review fixtures for the review race tests.

use super::*;

/// Candidate handle shared by the session projection and its review response.
pub(super) const CANDIDATE: &str = "cnd_review";

/// Candidate handle published by the newest session generation after a refresh.
pub(super) const SUPERSEDING_CANDIDATE: &str = "cnd_revised";

/// Session identity used by the overlapping candidate review requests.
///
/// # Returns
/// The stable session identity shared by the request generations.
pub(super) fn session_identity() -> scriberig_core::EntityId {
    scriberig_core::EntityId::from_bytes([21; 16])
}

/// Builds the session projection returned by one session-read generation.
///
/// - `candidate`: Durable candidate handle and its exact base snapshot, or `None` to
///   return a projection without any reviewable candidate row.
///
/// # Returns
/// A session projection for the shared session identity.
pub(super) fn session_projection(
    candidate: Option<(&str, scriberig_core::SnapshotId)>,
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
    if let Some((handle, snapshot)) = candidate {
        rows.push(crate::view::session::SessionRow {
            reference: branch,
            depth: 2,
            label: format!("Review candidate {handle}"),
            kind: crate::view::session::SessionRowKind::Candidate,
            candidate: Some(handle.to_owned()),
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
/// - `candidate`: Durable candidate handle carried by the review.
/// - `base_snapshot`: Exact base snapshot carried by the reviewed candidate.
///
/// # Returns
/// A passing review model for the requested candidate handle.
pub(super) fn review_response(
    candidate: &str,
    base_snapshot: scriberig_core::SnapshotId,
) -> crate::view::changes::ChangeReviewModel {
    crate::view::changes::ChangeReviewModel {
        base_snapshot,
        candidate_snapshot: None,
        change: candidate.to_owned(),
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
pub(super) fn state_before_candidate() -> TuiState {
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
