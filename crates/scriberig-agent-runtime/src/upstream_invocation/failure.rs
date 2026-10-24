//! Maps upstream provider and agent-loop terminal failures into `ScribeRig` categories.

use rpi_agent::AgentState;
use rpi_ai::types::{ErrorReason, StopReason};

use super::UpstreamBranchError;

/// Classifies one terminal provider failure with its upstream typed reason.
///
/// The upstream loop already performs provider transport and authorization
/// classification and exposes it as `AgentState::provider_error_reason`. This
/// adapter only preserves those distinctions in the public `ScribeRig`
/// categories; it never re-derives transport or authentication details.
///
/// # Parameters
/// - `reason`: Typed terminal reason recorded by the upstream loop.
/// - `stop_reason`: Terminal stop reason of the final assistant response.
/// - `message`: Safe failure text from the upstream loop or terminal message.
///
/// # Returns
/// The matching `ScribeRig` branch invocation failure.
pub(super) fn classify_provider_failure(
    reason: Option<ErrorReason>,
    stop_reason: StopReason,
    message: String,
) -> UpstreamBranchError {
    match reason.unwrap_or(match stop_reason {
        StopReason::Aborted => ErrorReason::Aborted,
        _ => ErrorReason::Error,
    }) {
        ErrorReason::Authentication => UpstreamBranchError::Authentication(message),
        ErrorReason::Aborted => UpstreamBranchError::Cancelled,
        ErrorReason::Error => UpstreamBranchError::Agent(message),
    }
}

/// Projects the terminal outcome recorded in one completed upstream agent state.
///
/// Upstream keeps the provider terminal reason separate from the assistant
/// message, so the state snapshot is the only source for authentication and
/// abort categories. A successful or non-terminal state yields `None` and stays
/// under the caller's missing-assistant handling.
///
/// # Parameters
/// - `state`: Final public state snapshot of the upstream agent run.
///
/// # Returns
/// The `ScribeRig` failure when the upstream run ended failed, aborted, or with
/// an incomplete terminal response; `None` otherwise.
pub(super) fn classify_agent_state(state: &AgentState) -> Option<UpstreamBranchError> {
    let reason = state.provider_error_reason;
    if let Some(message) = &state.error_message {
        return Some(classify_provider_failure(
            reason,
            StopReason::Error,
            message.clone(),
        ));
    }
    let assistant = state.messages.iter().rev().find_map(|m| m.as_assistant())?;
    if let Some(message) = &assistant.error_message {
        return Some(classify_provider_failure(
            reason,
            assistant.stop_reason,
            message.clone(),
        ));
    }
    match assistant.stop_reason {
        StopReason::Error | StopReason::Aborted => Some(classify_provider_failure(
            reason,
            assistant.stop_reason,
            "provider response did not complete successfully".to_owned(),
        )),
        _ => None,
    }
}

/// Classifies one upstream agent-loop failure returned by the loop entry point.
///
/// Upstream reports loop-level aborts separately from provider faults, and
/// `ScribeRig` keeps that separation so a cancelled run is never presented as a
/// provider failure.
///
/// # Parameters
/// - `error`: Upstream agent-loop failure.
///
/// # Returns
/// The matching `ScribeRig` branch invocation failure.
pub(super) fn classify_loop_failure(error: &rpi_agent::AgentError) -> UpstreamBranchError {
    if error.is_abort() {
        UpstreamBranchError::Cancelled
    } else {
        UpstreamBranchError::Agent(error.to_string())
    }
}
