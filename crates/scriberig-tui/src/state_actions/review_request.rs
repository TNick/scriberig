//! Starts candidate review generations without orphaning an outstanding request.

use super::*;

/// Starts one candidate review generation for the exact candidate request.
///
/// A durable session read can supersede the session while the candidate review is
/// already outstanding. The newest projection then reopens the same candidate, and a
/// second generation would discard the response that can still settle the review, so
/// an identical outstanding request keeps its generation instead of being re-issued.
///
/// - `state`: TUI state receiving the review generation.
/// - `request`: Exact candidate handle and base snapshot to review.
///
/// An Apply effect for the newest generation, or a no-op effect when the same
/// candidate request is already outstanding.
/// # Returns
/// The reducer effect that opens the candidate review.
pub(in crate::state) fn request_candidate_review(
    state: &mut TuiState,
    request: scriberig_app::ApplyRequest,
) -> Vec<Effect> {
    if matches!(state.review, LoadState::Loading { .. })
        && state.review_request.as_ref() == Some(&request)
    {
        return vec![Effect::None];
    }
    let request_id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    state.review = LoadState::Loading { request_id };
    state.review_request = Some(request.clone());
    vec![Effect::Apply {
        request_id,
        request,
    }]
}
