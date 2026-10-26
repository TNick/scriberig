//! Adapts remote model-library and execution-selection protocol calls.

use super::{ApplicationError, RemoteAdapter};
use scriberig_app::RuntimeCapability;
use scriberig_protocol::ProtocolTransport;

/// Reads runtime capability descriptors from the remote client.
///
/// - `adapter`: Remote service adapter holding the protocol client.
///
/// Decoded runtime capabilities.
///
/// Returns a protocol capability-discovery error.
pub(super) fn runtime_capabilities<T: ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
) -> Result<Vec<RuntimeCapability>, ApplicationError> {
    adapter
        .client
        .capabilities()
        .map_err(super::service_remote_mapping::remote_error)
        .map(|response| {
            response
                .capabilities
                .into_iter()
                .filter_map(|capability| {
                    Some(RuntimeCapability {
                        id: scriberig_app::CapabilityId::new(capability.id).ok()?,
                        description: capability.description,
                        available: capability.available,
                        scope: capability.domain,
                    })
                })
                .collect()
        })
}

/// Reads the safe model library through a remote protocol client.
///
/// - `adapter`: Remote service adapter holding the protocol client.
///
/// Safe upstream-derived model display rows.
///
/// Returns a sanitized remote protocol error.
pub(super) fn model_library<T: ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
    adapter
        .client
        .model_library()
        .map_err(super::service_remote_mapping::remote_error)
}

/// Reads bounded safe detail for a durable tool result.
///
/// - adapter: Remote service adapter holding the protocol client.
/// - session: Session containing the result.
/// - branch: Branch containing the result.
/// - result: Stable tool-result identity.
///
/// Whitelisted safe result detail.
///
/// Returns a sanitized remote protocol error.
pub(super) fn tool_result_detail<T: ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    session: scriberig_core::EntityId,
    branch: scriberig_core::EntityId,
    result: scriberig_core::EntityId,
) -> Result<scriberig_protocol::ToolResultDetailResponse, ApplicationError> {
    adapter
        .client
        .tool_result_detail(session.to_string(), branch.to_string(), result.to_string())
        .map_err(super::service_remote_mapping::remote_error)
}

/// Reads the captured safe request projection through the remote client.
///
/// - `adapter`: Remote adapter holding the protocol client.
/// - `session`: Session containing the invocation.
/// - `branch`: Branch containing the invocation.
/// - `invocation`: Stable model-invocation identity.
///
/// Captured provider-neutral request detail.
///
/// Returns a sanitized remote protocol error.
pub(super) fn model_request_detail<T: ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    session: scriberig_core::EntityId,
    branch: scriberig_core::EntityId,
    invocation: scriberig_core::EntityId,
) -> Result<scriberig_protocol::ModelRequestDetailResponse, ApplicationError> {
    let session = session.to_string();
    let branch = branch.to_string();
    let invocation = invocation.to_string();
    let mut offset = 0;
    let mut detail: Option<scriberig_protocol::ModelRequestDetailResponse> = None;
    let mut total_chars = None;

    loop {
        let page = adapter
            .client
            .model_request_detail_page(
                session.clone(),
                branch.clone(),
                invocation.clone(),
                offset,
                32_768,
            )
            .map_err(super::service_remote_mapping::remote_error)?;
        if page.invocation != invocation || page.offset != Some(offset) {
            return Err(ApplicationError::Internal {
                message: "remote model request detail returned an invalid page identity".to_owned(),
            });
        }
        match (total_chars, page.total_chars) {
            (Some(expected), Some(actual)) if expected == actual => {}
            (None, Some(actual)) => total_chars = Some(actual),
            _ => {
                return Err(ApplicationError::Internal {
                    message: "remote model request detail pages disagree on total length"
                        .to_owned(),
                });
            }
        }
        if let Some(detail) = &mut detail {
            detail.fragments.extend(page.fragments);
            detail.content_complete &= page.content_complete;
            detail.next_offset = page.next_offset;
        } else {
            detail = Some(page);
        }
        let next = detail.as_ref().and_then(|response| response.next_offset);
        let Some(next) = next else {
            break;
        };
        if next <= offset || total_chars.is_none_or(|total| next > total) {
            return Err(ApplicationError::Internal {
                message: "remote model request detail returned an invalid continuation".to_owned(),
            });
        }
        offset = next;
    }

    let mut detail = detail.ok_or_else(|| ApplicationError::Internal {
        message: "remote model request detail returned no page".to_owned(),
    })?;
    detail.offset = Some(0);
    detail.total_chars = total_chars;
    detail.next_offset = None;
    Ok(detail)
}

/// Applies a revision-checked provider/model selection remotely.
///
/// - `adapter`: Remote service adapter holding the protocol client.
/// - `request`: Session and observed selection revision.
///
/// The authoritative updated execution selection.
///
/// Returns a sanitized remote protocol error.
pub(super) fn set_session_model<T: ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &scriberig_app::SessionModelSelectionRequest,
) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
    adapter
        .client
        .set_session_model(
            request.session.to_string(),
            request.expected_revision,
            request.provider_id.clone(),
            request.model_id.clone(),
        )
        .map_err(super::service_remote_mapping::remote_error)
        .map(to_session_selection)
}

/// Applies a revision-checked thinking selection remotely.
///
/// - `adapter`: Remote service adapter holding the protocol client.
/// - `request`: Session and observed selection revision.
///
/// The authoritative updated execution selection.
///
/// Returns a sanitized remote protocol error.
pub(super) fn set_session_thinking_level<T: ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &scriberig_app::SessionThinkingSelectionRequest,
) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
    adapter
        .client
        .set_session_thinking_level(
            request.session.to_string(),
            request.expected_revision,
            request.thinking_level.clone(),
        )
        .map_err(super::service_remote_mapping::remote_error)
        .map(to_session_selection)
}

/// Converts a protocol selection into the session-domain value.
///
/// - `selection`: Protocol selection returned by the server.
///
/// Equivalent provider-neutral session selection.
/// # Returns
/// The operation result.
fn to_session_selection(
    selection: scriberig_protocol::SessionExecutionSelection,
) -> scriberig_session::SessionExecutionSelection {
    scriberig_session::SessionExecutionSelection {
        revision: selection.revision,
        provider_id: selection.provider_id,
        model_id: selection.model_id,
        thinking_level: selection.thinking_level,
        history: selection
            .history
            .into_iter()
            .map(|change| scriberig_session::SessionExecutionChange {
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
#[path = "service_remote_model_tests.rs"]
mod tests;
