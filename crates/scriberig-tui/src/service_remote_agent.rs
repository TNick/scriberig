//! Adapts remote agent submission calls to stable TUI handles.

use super::{ApplicationError, RemoteAdapter};
use scriberig_core::EntityId;
use scriberig_protocol::ProtocolTransport;
use std::str::FromStr;

/// Submits a remote agent command and resolves its operation identity.
///
/// # Parameters
/// - `adapter`: Remote service adapter holding the protocol client.
/// - `session`: Session receiving the submitted command.
/// - `branch`: Optional branch receiving the command.
/// - `prompt`: User-authored instruction.
///
/// # Returns
/// Stable operation identity accepted by the server.
///
/// # Errors
/// Returns a remote protocol or identity error.
pub(super) fn submit_agent_command<T: ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    session: EntityId,
    branch: Option<EntityId>,
    prompt: &str,
) -> Result<scriberig_core::OperationId, ApplicationError> {
    let operation = adapter
        .client
        .submit_agent_command(
            session.to_string(),
            branch.map(|value| value.to_string()),
            prompt,
        )
        .map_err(super::service_remote_mapping::remote_error)?;
    scriberig_core::OperationId::from_str(&operation.operation.reference.id).map_err(|error| {
        ApplicationError::Internal {
            message: format!("remote response contained invalid operation identity: {error}"),
        }
    })
}

/// Submits remote agent work and retains its accepted branch handle.
///
/// # Parameters
/// - `adapter`: Remote service adapter holding the protocol client.
/// - `session`: Session receiving the submitted command.
/// - `branch`: Optional continuation branch.
/// - `prompt`: User-authored instruction.
///
/// # Returns
/// Stable operation, session, and branch handles.
///
/// # Errors
/// Returns a sanitized remote protocol or identity error.
pub(super) fn submit_agent_handle<T: ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    session: EntityId,
    branch: Option<EntityId>,
    prompt: &str,
) -> Result<super::AgentHandle, ApplicationError> {
    let accepted = match branch {
        Some(branch) => adapter
            .client
            .continue_agent_with_handles(session.to_string(), branch.to_string(), prompt)
            .map_err(super::service_remote_mapping::remote_error)?,
        None => adapter
            .client
            .start_agent_with_handles(session.to_string(), prompt)
            .map_err(super::service_remote_mapping::remote_error)?,
    };
    let operation = scriberig_core::OperationId::from_str(
        &accepted.operation.operation.reference.id,
    )
    .map_err(|error| ApplicationError::Internal {
        message: format!("remote response contained invalid operation identity: {error}"),
    })?;
    let branch = accepted
        .branch
        .as_ref()
        .map(|value| EntityId::from_str(&value.id))
        .transpose()
        .map_err(|error| ApplicationError::Internal {
            message: format!("remote response contained invalid branch identity: {error}"),
        })?;
    Ok(super::AgentHandle {
        operation,
        session,
        branch,
        workspace: None,
    })
}
