//! Executes shared application commands for the protocol server.

use super::server_commands::decode_command;
use super::server_helpers::{operation_from_record, result_handles};
use crate::server::ApplicationService as _;
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, OperationId,
};
use scriberig_protocol::Operation;

/// Decodes and executes one command through the shared application boundary.
///
/// # Parameters
/// - `service`: Shared application service.
/// - `context`: Trusted request context.
/// - `kind`: Stable command family.
/// - `payload`: Typed command payload in JSON form.
///
/// # Returns
/// The accepted or completed shared operation.
///
/// # Errors
/// Returns a structured application failure.
pub(super) fn execute<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    kind: &str,
    payload: &serde_json::Value,
) -> Result<Operation, ApplicationError> {
    if matches!(kind, "agent_start" | "agent_continue") {
        return submit_agent_command(service, context, kind, payload);
    }
    let (operation_kind, request) = decode_command(kind, payload)?;
    let operation_id = OperationId::new(format!(
        "command-{kind}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?
            .as_nanos()
    ))
    .map_err(|error| ApplicationError::Internal {
        message: error.to_string(),
    })?;
    service
        .operations()
        .start(operation_id.clone(), operation_kind)?;
    service
        .operations()
        .transition(&operation_id, scriberig_app::OperationStatus::Running)?;
    let result = service.execute(context, &request);
    match result {
        Ok(result) => {
            let handles = result_handles(&result);
            Ok(operation_from_record(
                service.operations().complete(&operation_id, handles)?,
            ))
        }
        Err(error) => {
            let _ = service.operations().fail(&operation_id, error.clone());
            Err(error)
        }
    }
}

/// Parses and submits a protocol agent command through the shared session API.
///
/// # Parameters
/// - `service`: Shared application service.
/// - `context`: Trusted request context.
/// - `kind`: Agent command family.
/// - `payload`: Agent command payload in JSON form.
///
/// # Returns
/// The accepted agent operation.
///
/// # Errors
/// Returns a structured validation, authorization, or operation error.
fn submit_agent_command<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    kind: &str,
    payload: &serde_json::Value,
) -> Result<Operation, ApplicationError> {
    let session = payload
        .get("session")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ApplicationError::InvalidRequest {
            operation: kind.to_owned(),
            message: "session is required".to_owned(),
        })?
        .parse::<scriberig_core::EntityId>()
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: kind.to_owned(),
            message: format!("invalid session: {error}"),
        })?;
    let branch = payload
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .map(str::parse::<scriberig_core::EntityId>)
        .transpose()
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: kind.to_owned(),
            message: format!("invalid branch: {error}"),
        })?;
    let prompt = payload
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApplicationError::InvalidRequest {
            operation: kind.to_owned(),
            message: "prompt is required".to_owned(),
        })?;
    let selection = match payload
        .get("selection")
        .filter(|value| !value.is_null())
        .cloned()
    {
        None => None,
        Some(value) => {
            let selection = serde_json::from_value::<scriberig_protocol::AgentSelection>(value)
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: kind.to_owned(),
                    message: format!("invalid semantic selection: {error}"),
                })?;
            Some(scriberig_app::AgentSelection {
                project: selection
                    .project_id
                    .as_deref()
                    .map(str::parse)
                    .transpose()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: kind.to_owned(),
                        message: format!("invalid selected project: {error}"),
                    })?,
                root: selection
                    .root
                    .as_ref()
                    .map(|root| root.id.parse())
                    .transpose()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: kind.to_owned(),
                        message: format!("invalid selected root: {error}"),
                    })?
                    .unwrap_or(selection.entity.id.parse().map_err(|error| {
                        ApplicationError::InvalidRequest {
                            operation: kind.to_owned(),
                            message: format!("invalid selected entity: {error}"),
                        }
                    })?),
                entity: selection.entity.id.parse().map_err(|error| {
                    ApplicationError::InvalidRequest {
                        operation: kind.to_owned(),
                        message: format!("invalid selected entity: {error}"),
                    }
                })?,
                snapshot: selection.snapshot.id.parse().map_err(|error| {
                    ApplicationError::InvalidRequest {
                        operation: kind.to_owned(),
                        message: format!("invalid selected snapshot: {error}"),
                    }
                })?,
            })
        }
    };
    let operation = scriberig_app::SessionBackend::submit_agent_command(
        service,
        context,
        &scriberig_app::AgentCommandRequest {
            session,
            branch,
            prompt: prompt.to_owned(),
            selection,
        },
    )?;
    service.operation_status(&operation.to_string())
}
