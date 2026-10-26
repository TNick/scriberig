//! Covers the candidate review generation kept across a superseding session refresh.

use super::candidate_review_support::{
    CANDIDATE, review_response, session_identity, session_projection, state_before_candidate,
};
use super::*;

/// Verifies a superseding session refresh keeps the outstanding candidate review.
///
/// The durable session event reopens the candidate the outstanding Apply already asks
/// the application to prepare. Issuing a second generation would discard the response
/// that can still settle the review and leave it loading with a valid candidate, so the
/// refresh must reuse the outstanding request.
#[test]
fn superseding_session_refresh_reuses_the_outstanding_candidate_review() {
    let base = scriberig_core::SnapshotId::new([4; 32]);
    let mut state = state_before_candidate();

    let refresh = crate::state::activate_palette_item(&mut state, "review-candidate");
    assert!(matches!(
        refresh.as_slice(),
        [Effect::ReadSession { request_id: 1, .. }]
    ));
    let opened = reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 1,
            model: session_projection(Some((CANDIDATE, base))),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: None,
        },
    );
    assert!(matches!(
        opened.as_slice(),
        [Effect::Apply { request_id: 2, request }] if request.change == CANDIDATE && request.base_snapshot == base
    ));
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Loading { request_id: 2 }
    ));

    // The durable session event supersedes the session while that Apply is outstanding.
    let competing = reduce(
        &mut state,
        Action::SessionRequested {
            session: session_identity(),
            branch: None,
        },
    );
    assert!(matches!(
        competing.as_slice(),
        [Effect::ReadSession { request_id: 3, .. }]
    ));
    let reopened = reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 3,
            model: session_projection(Some((CANDIDATE, base))),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: None,
        },
    );
    assert_eq!(reopened, vec![Effect::None]);
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Loading { request_id: 2 }
    ));

    // The outstanding response still settles the review for the valid candidate.
    let settled = reduce(
        &mut state,
        Action::ReviewSucceeded {
            request_id: 2,
            model: review_response(CANDIDATE, base),
        },
    );
    assert_eq!(settled, vec![Effect::None]);
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Ready(_)
    ));
}

/// Verifies a failed review generation is requested again by the next refresh.
///
/// A failed generation has no outstanding response to reuse, so the newest session
/// projection carrying the same candidate must open a new review generation.
#[test]
fn failed_candidate_review_is_requested_again_by_the_next_refresh() {
    let base = scriberig_core::SnapshotId::new([4; 32]);
    let mut state = state_before_candidate();
    let refresh = crate::state::activate_palette_item(&mut state, "review-candidate");
    assert!(matches!(
        refresh.as_slice(),
        [Effect::ReadSession { request_id: 1, .. }]
    ));
    let opened = reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 1,
            model: session_projection(Some((CANDIDATE, base))),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: None,
        },
    );
    assert!(matches!(
        opened.as_slice(),
        [Effect::Apply { request_id: 2, .. }]
    ));
    reduce(
        &mut state,
        Action::OperationFailed {
            request_id: 2,
            category: "filesystem_conflict".to_owned(),
            message: "source changed externally: Cargo.toml".to_owned(),
        },
    );
    assert!(matches!(
        state.review,
        crate::inspector::LoadState::Failed { .. }
    ));

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
            model: session_projection(Some((CANDIDATE, base))),
            conversation: crate::view::conversation::ConversationModel::default(),
            session_snapshot: None,
        },
    );
    assert!(matches!(
        reopened.as_slice(),
        [Effect::Apply { request_id: 4, request }] if request.change == CANDIDATE
            && request.base_snapshot == base
    ));
}
