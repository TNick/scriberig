//! Adapts shared application model operations to protocol-safe DTOs.

use scriberig_app::{AgentConsoleBackend, ApplicationBackend, ApplicationContext};
use scriberig_protocol::{
    ModelLibraryResponse, PublicModelCostSnapshot, PublicModelCostTierSnapshot,
    PublicModelDescriptor, PublicModelInputModality, PublicModelMetadataSnapshot,
    SessionExecutionSelection,
};

#[path = "server_agent_console/recovery.rs"]
mod recovery;
pub(super) use recovery::{
    legacy_execution_records, map_legacy_execution_selection, map_legacy_plan_assignment,
};

/// Reads the upstream-derived model compatibility projection.
///
/// - `self`: Shared application service.
/// - `context`: Trusted capability context.
///
/// Protocol-safe upstream-derived model display rows.
///
/// Returns authorization or application configuration failures.
pub(super) fn model_library<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
) -> Result<ModelLibraryResponse, scriberig_app::ApplicationError> {
    let models = AgentConsoleBackend::agent_model_library(service, context)?;
    Ok(ModelLibraryResponse {
        provider_readiness: Vec::new(),
        models: models
            .into_iter()
            .map(|model| PublicModelDescriptor {
                id: scriberig_protocol::model_row_id(&model.provider_id, &model.model_id),
                display_name: model.display_name,
                provider_id: model.provider_id,
                provider_model_id: model.model_id,
                thinking_levels: model
                    .thinking_levels
                    .into_iter()
                    .map(|level| level.as_str().to_owned())
                    .collect(),
                metadata_snapshot: PublicModelMetadataSnapshot {
                    schema_version: model.schema_version,
                    api_id: model.api_id,
                    reasoning: model.reasoning,
                    input: model
                        .input
                        .into_iter()
                        .map(|modality| match modality {
                            rpi_ai::InputModality::Text => PublicModelInputModality::Text,
                            rpi_ai::InputModality::Image => PublicModelInputModality::Image,
                        })
                        .collect(),
                    cost: PublicModelCostSnapshot {
                        input: model.cost.rates.input,
                        output: model.cost.rates.output,
                        cache_read: model.cost.rates.cache_read,
                        cache_write: model.cost.rates.cache_write,
                        tiers: model
                            .cost
                            .tiers
                            .into_iter()
                            .map(|tier| PublicModelCostTierSnapshot {
                                input: tier.rates.input,
                                output: tier.rates.output,
                                cache_read: tier.rates.cache_read,
                                cache_write: tier.rates.cache_write,
                                input_tokens_above: tier.input_tokens_above,
                            })
                            .collect(),
                    },
                    cost_configured: model.cost_configured,
                    context_window: model.context_window,
                    max_tokens: model.max_tokens,
                },
            })
            .collect(),
    })
}

/// Selects an upstream provider/model pair for future invocations.
///
/// - `self`: Shared application service.
/// - `context`: Trusted capability context.
/// - `session`: Serialized stable session identity.
/// - `expected_revision`: Selection revision observed by the client.
/// - `provider_id`: Exact upstream provider identity.
/// - `model_id`: Provider-scoped upstream model identity.
///
/// Updated provider-neutral session selection.
///
/// Returns authorization, validation, conflict, or persistence failures.
pub(super) fn set_session_model<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    session: &str,
    expected_revision: u64,
    provider_id: &str,
    model_id: &str,
) -> Result<SessionExecutionSelection, scriberig_app::ApplicationError> {
    let session = parse_session(session)?;
    let selection = AgentConsoleBackend::set_session_model(
        service,
        context,
        &scriberig_app::SessionModelSelectionRequest {
            session,
            expected_revision,
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
        },
    )?;
    Ok(selection_response(selection))
}

/// Selects a thinking level for future invocations.
///
/// - `self`: Shared application service.
/// - `context`: Trusted capability context.
/// - `session`: Serialized stable session identity.
/// - `expected_revision`: Selection revision observed by the client.
/// - `thinking_level`: Advertised thinking capability identity.
///
/// Updated provider-neutral session selection.
///
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
/// - `service`: Application service owning session state.
/// - `context`: Trusted session-read context.
/// - `session`: Serialized session identity.
/// - `branch`: Serialized branch identity.
/// - `invocation`: Serialized model invocation identity.
/// - `offset`: Optional Unicode-scalar cursor for full detail paging.
/// - `max_chars`: Optional maximum page character count.
///
/// Bounded request data captured before provider serialization.
///
/// Returns invalid-request or not-found when any selected record is absent.
pub(super) fn model_request_detail<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    session: &str,
    branch: &str,
    invocation: &str,
    offset: Option<u64>,
    max_chars: Option<u32>,
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
    Ok(project_model_request(
        invocation_id.to_string(),
        &request,
        offset,
        max_chars,
    ))
}

/// Reads expanded bounded detail for one durable tool result.
///
/// - `service`: Application service owning session state.
/// - `context`: Trusted session-read context.
/// - `session`: Serialized session identity.
/// - `branch`: Serialized branch identity.
/// - `result`: Serialized tool result identity.
///
/// Whitelisted safe result detail bounded to 32,768 characters.
///
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
/// - `value`: Serialized session identity.
///
/// The typed stable session identity.
///
/// Returns invalid-request when the identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `value`: Serialized entity identity.
/// - `operation`: Operation name used in the validation error.
///
/// The typed stable entity identity.
///
/// Returns invalid-request when the identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `invocation`: Stable model invocation identity.
/// - `request`: Captured safe request projection.
/// - `offset`: Optional Unicode-scalar offset for a detail page.
/// - `max_chars`: Optional detail-page character limit.
///
/// Protocol detail with provider-neutral fields only.
/// # Returns
/// The operation result.
fn project_model_request(
    invocation: String,
    request: &scriberig_session::ModelRequestProjection,
    offset: Option<u64>,
    max_chars: Option<u32>,
) -> scriberig_protocol::ModelRequestDetailResponse {
    let mut response = scriberig_protocol::ModelRequestDetailResponse {
        invocation,
        model: request.model.clone(),
        provider_id: request.provider_id.clone(),
        provider_profile: request.provider_profile.clone(),
        thinking_level: request.thinking_level.clone(),
        profile: request.profile.as_ref().map(|reference| {
            scriberig_protocol::ModelContextRevisionReference {
                identity: reference.identity.clone(),
                revision: reference.revision.clone(),
            }
        }),
        active_components: request
            .active_components
            .iter()
            .map(
                |reference| scriberig_protocol::ModelContextRevisionReference {
                    identity: reference.identity.clone(),
                    revision: reference.revision.clone(),
                },
            )
            .collect(),
        context_fingerprint: request.context_fingerprint.clone(),
        request_fingerprint: request.request_fingerprint.clone(),
        catalog_revision: request.catalog_revision,
        upstream_runtime_revision: request.upstream_runtime_revision.clone(),
        provider_response_id: request.provider_response_id.clone(),
        model_execution_configuration: request.model_execution_configuration.clone(),
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
        applied_settings: request.applied_settings.clone(),
        applied_settings_fingerprint: request.applied_settings_fingerprint.clone(),
        truncated: request.truncated,
        content_complete: request.content_complete,
        fragments: Vec::new(),
        offset: None,
        total_chars: None,
        next_offset: None,
    };
    if offset.is_some() || max_chars.is_some() {
        let page = scriberig_app::agent_console::request_page::model_request_detail_page(
            request,
            offset.unwrap_or(0),
            max_chars.unwrap_or(16_384),
        );
        response.fragments = page
            .fragments
            .into_iter()
            .map(|fragment| scriberig_protocol::ModelRequestDetailFragment {
                kind: fragment.kind,
                index: fragment.index,
                parent_index: fragment.parent_index,
                field: fragment.field,
                field_offset: fragment.field_offset,
                role: fragment.role,
                text: fragment.text,
            })
            .collect();
        response.content_complete = page.content_complete;
        response.offset = Some(page.offset);
        response.total_chars = Some(page.total_chars);
        response.next_offset = page.next_offset;
    }
    response
}

/// Converts the shared selection into its public protocol projection.
///
/// - `selection`: Shared session execution selection.
///
/// Protocol-safe selection values.
/// # Returns
/// The operation result.
pub(super) fn selection_response(
    selection: scriberig_session::SessionExecutionSelection,
) -> SessionExecutionSelection {
    SessionExecutionSelection {
        revision: selection.revision,
        provider_id: selection.provider_id,
        model_id: selection.model_id,
        thinking_level: selection.thinking_level,
        history: selection
            .history
            .into_iter()
            .map(|change| scriberig_protocol::SessionExecutionChange {
                revision: change.revision,
                kind: change.kind,
                summary: change.summary,
                provider_id: change.provider_id,
                model_id: change.model_id,
                thinking_level: change.thinking_level,
            })
            .collect(),
    }
}

#[cfg(test)]
#[path = "server_agent_console_model_projection_tests.rs"]
mod model_projection_tests;
#[cfg(test)]
#[path = "server_agent_console_tests.rs"]
mod tests;
