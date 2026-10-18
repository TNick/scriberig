//! Dispatches model and presentation preference operations.

use super::helpers;
use crate::server::ApplicationService;
use scriberig_app::{ApplicationContext, CapabilityContext, CapabilityId, Principal};
use scriberig_protocol::{
    AgentPresentationPreferencesSetRequest, Response, SessionModelSelectionRequest,
    SessionThinkingSelectionRequest,
};

/// Reads the safe model library.
///
/// # Parameters
/// - `service`: Application service handling model configuration.
/// - `context`: Trusted model-library capability context.
/// - `correlation_id`: Request correlation identity.
///
/// # Returns
/// Safe model descriptors or a structured protocol error.
pub(super) fn model_library<S: ApplicationService>(
    service: &S,
    context: &ApplicationContext,
    correlation_id: &str,
) -> Response {
    service.model_library(context).map_or_else(
        |error| Response::Error(helpers::map_error(&error, correlation_id)),
        Response::ModelLibrary,
    )
}

/// Reads shared preferences for the trusted principal.
///
/// # Parameters
/// - `service`: Application service owning shared preferences.
/// - `context`: Trusted capability and preference-owner context.
/// - `correlation_id`: Request correlation identity.
///
/// # Returns
/// Revisioned shared preferences or a structured protocol error.
pub(super) fn read_preferences<S: ApplicationService>(
    service: &S,
    context: &ApplicationContext,
    correlation_id: &str,
) -> Response {
    service.agent_presentation_preferences(context).map_or_else(
        |error| Response::Error(helpers::map_error(&error, correlation_id)),
        Response::AgentPresentationPreferences,
    )
}

/// Replaces shared preferences at the observed revision.
///
/// # Parameters
/// - `service`: Application service owning shared preferences.
/// - `context`: Trusted capability and preference-owner context.
/// - `request`: Expected revision and complete display choices.
/// - `correlation_id`: Request correlation identity.
///
/// # Returns
/// Updated preferences or a structured protocol error.
pub(super) fn set_preferences<S: ApplicationService>(
    service: &S,
    context: &ApplicationContext,
    request: &AgentPresentationPreferencesSetRequest,
    correlation_id: &str,
) -> Response {
    service
        .set_agent_presentation_preferences(context, request)
        .map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::AgentPresentationPreferences,
        )
}

/// Selects a model profile for future invocations.
///
/// # Parameters
/// - `service`: Application service handling session selection.
/// - `request`: Session, observed revision, and selected profile.
/// - `correlation_id`: Request correlation identity.
///
/// # Returns
/// Updated execution selection or a structured protocol error.
pub(super) fn set_model<S: ApplicationService>(
    service: &S,
    request: &SessionModelSelectionRequest,
    correlation_id: &str,
) -> Response {
    service
        .set_session_model(
            &super::session_selection_context(&request.session.id),
            &request.session.id,
            request.expected_revision,
            &request.model_profile,
        )
        .map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::SessionExecution,
        )
}

/// Selects a thinking level for future invocations.
///
/// # Parameters
/// - `service`: Application service handling session selection.
/// - `request`: Session, observed revision, and selected thinking level.
/// - `correlation_id`: Request correlation identity.
///
/// # Returns
/// Updated execution selection or a structured protocol error.
pub(super) fn set_thinking<S: ApplicationService>(
    service: &S,
    request: &SessionThinkingSelectionRequest,
    correlation_id: &str,
) -> Response {
    service
        .set_session_thinking_level(
            &super::session_selection_context(&request.session.id),
            &request.session.id,
            request.expected_revision,
            &request.thinking_level,
        )
        .map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::SessionExecution,
        )
}

/// Creates a trusted context for shared model preferences.
///
/// # Parameters
/// - `principal`: Caller identity established by the composition root.
///
/// # Returns
/// Capability-limited context with a separate user preference owner.
pub(super) fn presentation_context(principal: Option<&Principal>) -> ApplicationContext {
    ApplicationContext {
        principal: principal.cloned(),
        user_preferences_principal: principal.map(|value| value.id.clone()),
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_model_library").expect("model library capability")
        ]),
        ..ApplicationContext::empty()
    }
}
