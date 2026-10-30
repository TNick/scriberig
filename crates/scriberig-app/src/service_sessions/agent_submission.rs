//! Accepts authorized agent commands through the configured session backend.

use super::super::{ApplicationBackend, ApplicationService};
use crate::{AgentCommandRequest, ApplicationContext, ApplicationError};
use scriberig_core::OperationId;

use super::super::SessionBackend;
use super::service_session_authorization::authorize_agent_command;

/// Accepts one authorized agent command and tracks its operation.
///
/// # Parameters
/// - service: Application service owning session state.
/// - context: Authorized caller context.
/// - request: Agent command to submit.
///
/// The accepted operation identity.
///
/// Returns authorization, anchor, backend, or local acceptance failures.
/// # Returns
/// The accepted operation.
/// # Errors
/// Returns an error when the command cannot be accepted.
pub(super) fn submit<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &AgentCommandRequest,
) -> Result<OperationId, ApplicationError> {
    authorize_agent_command(context, request)?;
    if let Some(anchor) = request.anchor {
        if request.branch.is_some() {
            return Err(ApplicationError::InvalidRequest {
                operation: "agent_continue_at".to_owned(),
                message: "an exact anchor cannot be combined with a branch-head target".to_owned(),
            });
        }
        let session = service.read_session(
            context,
            &crate::SessionRequest {
                session: request.session,
            },
        )?;
        session
            .resolve_anchor(anchor)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "agent_continue_at".to_owned(),
                message: error.to_string(),
            })?;
    }
    let operation = match service.backend.submit_agent_command(context, request) {
        Ok(operation) => operation,
        Err(error) if error.category() == "unsupported_operation" => {
            service.accept_local_agent(request)?
        }
        Err(error) => return Err(error),
    };
    if service.operations.get(&operation).is_err() {
        service
            .operations
            .start(operation.clone(), crate::OperationKind::Agent)?;
        service
            .operations
            .transition(&operation, crate::OperationStatus::Running)?;
    }
    Ok(operation)
}
