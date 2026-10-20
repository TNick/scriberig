//! Applies failure diagnostics to the matching pending TUI model.

use super::*;

/// Marks the matching request as failed and preserves a visible error diagnostic.
///
/// - `state`: TUI models receiving the failure.
/// - `request_id`: Request generation associated with the failure.
/// - `category`: Stable application error category.
/// - `message`: Sanitized diagnostic for the user.
/// - `super`: Parent reducer module visibility marker.
///
/// A no-op effect after updating matching request state.
/// # Returns
/// The operation result.
pub(super) fn operation_failed(
    state: &mut TuiState,
    request_id: u64,
    category: &str,
    message: &str,
) -> Vec<Effect> {
    if category == "transport_unavailable" {
        state.connection = ConnectionState::Disconnected;
        state.stale = true;
    }
    state.operation_error = Some((category.to_owned(), message.to_owned()));
    let model_library_pending = matches!(
        &state.model_library,
        LoadState::Loading {
            request_id: current
        } if *current == request_id
    );
    if request_id == 0 {
        state.conversation_composer.pending = false;
        state.conversation_composer.error = Some(message.to_owned());
    }
    if request_id == 0
        && let Some(composer) = state.session_composer.as_mut()
        && let Some(pending) = composer.pending_prompt.take()
    {
        if composer.text.is_empty() {
            composer.text = pending;
        } else {
            composer.text = format!("{pending}\n{}", composer.text);
        }
        composer.error = Some(message.to_owned());
    }
    mark_failed(&mut state.history, request_id, category, message, false);
    mark_failed(&mut state.find, request_id, category, message, false);
    mark_failed(&mut state.session, request_id, category, message, false);
    mark_failed(
        &mut state.model_library,
        request_id,
        category,
        message,
        false,
    );
    if model_library_pending {
        state.conversation_composer.error = Some(message.to_owned());
    }
    mark_failed(&mut state.diff, request_id, category, message, false);
    mark_failed(&mut state.projection, request_id, category, message, false);
    let review_is_stale = state.review_proposal.is_some();
    mark_failed(
        &mut state.review,
        request_id,
        category,
        message,
        review_is_stale,
    );
    vec![Effect::None]
}

/// Marks a loading view failed only when its request generation matches.
///
/// - `view`: Presentation state to update.
/// - `request_id`: Request generation that failed.
/// - `category`: Stable application error category.
/// - `message`: Sanitized diagnostic for the user.
/// - `stale`: Whether a failed review retains a previous proposal.
fn mark_failed<T>(
    view: &mut LoadState<T>,
    request_id: u64,
    category: &str,
    message: &str,
    stale: bool,
) {
    if matches!(view, LoadState::Loading { request_id: current } if *current == request_id) {
        *view = LoadState::Failed {
            category: category.to_owned(),
            message: message.to_owned(),
            stale,
        };
    }
}
