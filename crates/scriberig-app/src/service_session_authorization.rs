//! Authorizes agent submission at the application session boundary.

use crate::{ApplicationContext, ApplicationError};

/// Authorizes one public agent submission before state changes.
///
/// # Parameters
/// - `super`: Parent module namespace.
/// - `context`: Caller capabilities and exact request context.
/// - `request`: Session and branch targeted by the command.
///
/// # Returns
/// Unit when the caller has the operation-specific capability.
///
/// # Errors
/// Returns permission-denied when the caller has no matching grant.
pub(super) fn authorize_agent_command(
    context: &ApplicationContext,
    request: &crate::AgentCommandRequest,
) -> Result<(), ApplicationError> {
    let capability = if request.branch.is_some() {
        "agent_continue"
    } else {
        "agent_start"
    };
    let capability_id =
        crate::CapabilityId::new(capability).map_err(|error| ApplicationError::Internal {
            message: error.to_owned(),
        })?;
    let action = crate::CapabilityAction::Custom(capability.to_owned());
    let scope = crate::CapabilityScope::Session(request.session.to_string());
    if context.capabilities.allows(&capability_id)
        || context.capabilities.permits(
            &action,
            &scope,
            context.snapshot,
            Some(crate::OperationKind::Agent),
        )
        || context.capabilities.permits(
            &crate::CapabilityAction::ModifyBranch,
            &scope,
            context.snapshot,
            Some(crate::OperationKind::Agent),
        )
    {
        return Ok(());
    }
    Err(ApplicationError::PermissionDenied {
        action: capability.to_owned(),
        scope: format!("session: {}", request.session),
    })
}

/// Authorizes one public session-tree read before exposing branch state.
///
/// # Parameters
/// - `super`: Parent module namespace.
/// - `context`: Caller capabilities and exact request context.
/// - `request`: Session identity targeted by the read.
///
/// # Returns
/// Unit when the caller has the session-tree capability.
///
/// # Errors
/// Returns permission-denied when the caller has no matching grant.
pub(super) fn authorize_session_read(
    context: &ApplicationContext,
    request: &crate::SessionRequest,
) -> Result<(), ApplicationError> {
    let capability = crate::CapabilityId::new("session_branch_tree").map_err(|error| {
        ApplicationError::Internal {
            message: error.to_owned(),
        }
    })?;
    let scope = crate::CapabilityScope::Session(request.session.to_string());
    if context.capabilities.allows(&capability)
        || context.capabilities.permits(
            &crate::CapabilityAction::Custom("session_branch_tree".to_owned()),
            &scope,
            context.snapshot,
            Some(crate::OperationKind::Agent),
        )
        || context.capabilities.permits(
            &crate::CapabilityAction::Inspect,
            &scope,
            context.snapshot,
            Some(crate::OperationKind::Agent),
        )
    {
        return Ok(());
    }
    Err(ApplicationError::PermissionDenied {
        action: "session_branch_tree".to_owned(),
        scope: format!("session: {}", request.session),
    })
}
