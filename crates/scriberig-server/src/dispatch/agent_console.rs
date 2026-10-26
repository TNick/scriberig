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
/// - `service`: Application service handling model configuration.
/// - `context`: Trusted model-library capability context.
/// - `correlation_id`: Request correlation identity.
///
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
/// - `service`: Application service owning shared preferences.
/// - `context`: Trusted capability and preference-owner context.
/// - `correlation_id`: Request correlation identity.
///
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
/// - `service`: Application service owning shared preferences.
/// - `context`: Trusted capability and preference-owner context.
/// - `request`: Expected revision and complete display choices.
/// - `correlation_id`: Request correlation identity.
///
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

/// Selects an upstream provider/model pair for future invocations.
///
/// - `service`: Application service handling session selection.
/// - `request`: Session, observed revision, and upstream provider/model IDs.
/// - `correlation_id`: Request correlation identity.
///
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
            &request.provider_id,
            &request.model_id,
        )
        .map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::SessionExecution,
        )
}

/// Selects a thinking level for future invocations.
///
/// - `service`: Application service handling session selection.
/// - `request`: Session, observed revision, and selected thinking level.
/// - `correlation_id`: Request correlation identity.
///
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

/// Reads preserved legacy execution records for operator recovery.
///
/// - `service`: Application service handling the recovery query.
/// - `request`: Session identity to inspect.
/// - `correlation_id`: Request correlation identity.
///
/// Sanitized legacy records or a structured protocol error.
/// # Returns
/// The operation result.
pub(super) fn legacy_execution_records<S: ApplicationService>(
    service: &S,
    request: &scriberig_protocol::LegacyExecutionRecordsRequest,
    correlation_id: &str,
) -> Response {
    service
        .legacy_execution_records(
            &super::session_selection_context(&request.session.id),
            &request.session.id,
        )
        .map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::LegacyExecutionRecords,
        )
}

/// Maps one preserved legacy session selection to a current model.
///
/// - `service`: Application service handling the recovery operation.
/// - `request`: Exact legacy path and operator-selected current model.
/// - `correlation_id`: Request correlation identity.
///
/// Updated session execution selection or a structured protocol error.
/// # Returns
/// The operation result.
pub(super) fn map_legacy_execution_selection<S: ApplicationService>(
    service: &S,
    request: &scriberig_protocol::LegacyExecutionMappingRequest,
    correlation_id: &str,
) -> Response {
    service
        .map_legacy_execution_selection(
            &super::session_selection_context(&request.session.id),
            request,
        )
        .map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::SessionExecution,
        )
}

/// Maps one current legacy plan assignment through the shared application boundary.
///
/// - `service`: Application service handling plan assignments.
/// - `request`: Exact recovery path, observed plan revision, and selected model.
/// - `correlation_id`: Request correlation identity.
///
/// Updated plan projection or a structured protocol error.
/// # Returns
/// The operation result.
pub(super) fn map_legacy_plan_assignment<S: ApplicationService>(
    service: &S,
    request: &scriberig_protocol::LegacyPlanAssignmentMappingRequest,
    correlation_id: &str,
) -> Response {
    let available = service
        .capabilities()
        .iter()
        .any(|capability| capability.id.as_str() == "agent_plan" && capability.available);
    if !available {
        return Response::Error(helpers::map_error(
            &scriberig_app::ApplicationError::UnsupportedOperation {
                operation: "agent_plan".to_owned(),
            },
            correlation_id,
        ));
    }
    service
        .map_legacy_plan_assignment(&super::command_context("agent_plan"), request)
        .map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::Plan,
        )
}

/// Creates a trusted context for shared model preferences.
///
/// - `principal`: Caller identity established by the composition root.
///
/// Capability-limited context with a separate user preference owner.
/// # Returns
/// The operation result.
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
