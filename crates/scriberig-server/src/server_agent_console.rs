//! Adapts shared application model operations to protocol-safe DTOs.

use scriberig_app::{AgentConsoleBackend, ApplicationBackend, ApplicationContext};
use scriberig_protocol::{
    ModelLibraryResponse, PublicModelCapabilities, PublicModelDescriptor, PublicModelProfile,
    PublicThinkingLevelDescriptor, SessionExecutionSelection,
};

/// Reads the configured credential-safe model library.
///
/// # Parameters
/// - `self`: Shared application service.
/// - `context`: Trusted capability context.
///
/// # Returns
/// Protocol-safe model descriptors and profiles.
///
/// # Errors
/// Returns authorization or application configuration failures.
pub(super) fn model_library<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
) -> Result<ModelLibraryResponse, scriberig_app::ApplicationError> {
    let library = AgentConsoleBackend::agent_model_library(service, context)?;
    Ok(ModelLibraryResponse {
        models: library
            .models
            .into_iter()
            .map(|model| PublicModelDescriptor {
                id: model.id.as_str().to_owned(),
                display_name: model.display_name,
                provider_profile: model.provider_profile,
                provider_model_id: model.provider_model_id,
                capabilities: PublicModelCapabilities {
                    streaming: model.capabilities.streaming,
                    tool_calls: model.capabilities.tool_calls,
                    structured_output: model.capabilities.structured_output,
                },
                thinking_levels: model
                    .thinking_levels
                    .into_iter()
                    .map(|level| PublicThinkingLevelDescriptor {
                        id: level.id.as_str().to_owned(),
                        label: level.label,
                        description: level.description,
                        order: level.order,
                    })
                    .collect(),
                default_thinking_level: model
                    .default_thinking_level
                    .map(|value| value.as_str().to_owned()),
            })
            .collect(),
        profiles: library
            .profiles
            .into_iter()
            .map(|profile| PublicModelProfile {
                id: profile.id,
                display_name: profile.display_name,
                model: profile.model.as_str().to_owned(),
                connection: profile.connection,
                credential: profile
                    .credential
                    .map(|credential| scriberig_protocol::ModelCredentialRef { id: credential.id }),
                default_thinking_level: profile
                    .default_thinking_level
                    .map(|value| value.as_str().to_owned()),
                enabled: profile.enabled,
            })
            .collect(),
    })
}

/// Selects a model profile for future invocations.
///
/// # Parameters
/// - `self`: Shared application service.
/// - `context`: Trusted capability context.
/// - `session`: Serialized stable session identity.
/// - `expected_revision`: Selection revision observed by the client.
/// - `model_profile`: Curated profile identity.
///
/// # Returns
/// Updated provider-neutral session selection.
///
/// # Errors
/// Returns authorization, validation, conflict, or persistence failures.
pub(super) fn set_session_model<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    session: &str,
    expected_revision: u64,
    model_profile: &str,
) -> Result<SessionExecutionSelection, scriberig_app::ApplicationError> {
    let session = parse_session(session)?;
    let selection = AgentConsoleBackend::set_session_model(
        service,
        context,
        &scriberig_app::SessionModelSelectionRequest {
            session,
            expected_revision,
            model_profile: model_profile.to_owned(),
        },
    )?;
    Ok(selection_response(selection))
}

/// Selects a thinking level for future invocations.
///
/// # Parameters
/// - `self`: Shared application service.
/// - `context`: Trusted capability context.
/// - `session`: Serialized stable session identity.
/// - `expected_revision`: Selection revision observed by the client.
/// - `thinking_level`: Advertised thinking capability identity.
///
/// # Returns
/// Updated provider-neutral session selection.
///
/// # Errors
/// Returns authorization, validation, conflict, or persistence failures.
pub(super) fn set_session_thinking_level<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    session: &str,
    expected_revision: u64,
    thinking_level: &str,
) -> Result<SessionExecutionSelection, scriberig_app::ApplicationError> {
    let session = parse_session(session)?;
    let selection = AgentConsoleBackend::set_session_thinking_level(
        service,
        context,
        &scriberig_app::SessionThinkingSelectionRequest {
            session,
            expected_revision,
            thinking_level: thinking_level.to_owned(),
        },
    )?;
    Ok(selection_response(selection))
}

/// Reads the captured safe request for one durable invocation.
///
/// # Parameters
/// - `service`: Application service owning session state.
/// - `context`: Trusted session-read context.
/// - `session`: Serialized session identity.
/// - `branch`: Serialized branch identity.
/// - `invocation`: Serialized model invocation identity.
///
/// # Returns
/// Bounded request data captured before provider serialization.
///
/// # Errors
/// Returns invalid-request or not-found when any selected record is absent.
pub(super) fn model_request_detail<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    session: &str,
    branch: &str,
    invocation: &str,
) -> Result<scriberig_protocol::ModelRequestDetailResponse, scriberig_app::ApplicationError> {
    let session_id = parse_session(session)?;
    let branch_id = parse_entity(branch, "model request detail")?;
    let invocation_id = parse_entity(invocation, "model request detail")?;
    let request = scriberig_app::AgentConsoleBackend::model_request_detail(
        service,
        context,
        session_id,
        branch_id,
        invocation_id,
    )?;
    Ok(project_model_request(invocation_id.to_string(), &request))
}

/// Reads expanded bounded detail for one durable tool result.
///
/// # Parameters
/// - `service`: Application service owning session state.
/// - `context`: Trusted session-read context.
/// - `session`: Serialized session identity.
/// - `branch`: Serialized branch identity.
/// - `result`: Serialized tool result identity.
///
/// # Returns
/// Whitelisted safe result detail bounded to 32,768 characters.
///
/// # Errors
/// Returns invalid-request or not-found when a selected record is absent.
pub(super) fn tool_result_detail<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    session: &str,
    branch: &str,
    result: &str,
) -> Result<scriberig_protocol::ToolResultDetailResponse, scriberig_app::ApplicationError> {
    let session_id = parse_session(session)?;
    let branch_id = parse_entity(branch, "tool result detail")?;
    let result_id = parse_entity(result, "tool result detail")?;
    let detail = scriberig_app::AgentConsoleBackend::tool_result_detail(
        service, context, session_id, branch_id, result_id,
    )?;
    Ok(scriberig_protocol::ToolResultDetailResponse {
        result: detail.result.to_string(),
        detail: detail.detail,
        truncated: detail.truncated,
    })
}

/// Parses a stable entity id from a protocol session reference.
///
/// # Parameters
/// - `value`: Serialized session identity.
///
/// # Returns
/// The typed stable session identity.
///
/// # Errors
/// Returns invalid-request when the identity is malformed.
fn parse_session(value: &str) -> Result<scriberig_core::EntityId, scriberig_app::ApplicationError> {
    value
        .parse()
        .map_err(|error| scriberig_app::ApplicationError::InvalidRequest {
            operation: "session execution selection".to_owned(),
            message: format!("invalid session identifier: {error}"),
        })
}

/// Parses a branch or invocation entity id from a public reference.
///
/// # Parameters
/// - `value`: Serialized entity identity.
/// - `operation`: Operation name used in the validation error.
///
/// # Returns
/// The typed stable entity identity.
///
/// # Errors
/// Returns invalid-request when the identity is malformed.
fn parse_entity(
    value: &str,
    operation: &str,
) -> Result<scriberig_core::EntityId, scriberig_app::ApplicationError> {
    value
        .parse()
        .map_err(|error| scriberig_app::ApplicationError::InvalidRequest {
            operation: operation.to_owned(),
            message: format!("invalid entity identifier: {error}"),
        })
}

/// Converts a durable request projection into its bounded public DTO.
///
/// # Parameters
/// - `invocation`: Stable model invocation identity.
/// - `request`: Captured safe request projection.
///
/// # Returns
/// Protocol detail with provider-neutral fields only.
fn project_model_request(
    invocation: String,
    request: &scriberig_session::ModelRequestProjection,
) -> scriberig_protocol::ModelRequestDetailResponse {
    scriberig_protocol::ModelRequestDetailResponse {
        invocation,
        model: request.model.clone(),
        thinking_level: request.thinking_level.clone(),
        messages: request
            .messages
            .iter()
            .map(|message| scriberig_protocol::ModelRequestMessageDetail {
                role: message.role.clone(),
                content: message.content.clone(),
            })
            .collect(),
        tools: request
            .tools
            .iter()
            .map(|tool| scriberig_protocol::ModelRequestToolDetail {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: tool.parameters.clone(),
            })
            .collect(),
        truncated: request.truncated,
    }
}

/// Converts the shared selection into its public protocol projection.
///
/// # Parameters
/// - `selection`: Shared session execution selection.
///
/// # Returns
/// Protocol-safe selection values.
pub(super) fn selection_response(
    selection: scriberig_session::SessionExecutionSelection,
) -> SessionExecutionSelection {
    SessionExecutionSelection {
        revision: selection.revision,
        model_profile: selection.model_profile,
        model: selection.model,
        thinking_level: selection.thinking_level,
    }
}

#[cfg(test)]
#[path = "server_agent_console_tests.rs"]
mod tests;
