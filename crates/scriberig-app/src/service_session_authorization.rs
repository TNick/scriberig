//! Authorizes agent submission at the application session boundary.

use crate::{ApplicationContext, ApplicationError};

/// Authorizes one public agent submission before state changes.
///
/// - `super`: Parent module namespace.
/// - `context`: Caller capabilities and exact request context.
/// - `request`: Session and branch targeted by the command.
///
/// Unit when the caller has the operation-specific capability.
///
/// Returns permission-denied when the caller has no matching grant.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn authorize_agent_command(
    context: &ApplicationContext,
    request: &crate::AgentCommandRequest,
) -> Result<(), ApplicationError> {
    let capability = if request.branch.is_some() || request.anchor.is_some() {
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
/// - `super`: Parent module namespace.
/// - `context`: Caller capabilities and exact request context.
/// - `request`: Session identity targeted by the read.
///
/// Unit when the caller has the session-tree capability.
///
/// Returns permission-denied when the caller has no matching grant.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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

/// Authorizes a compact plan read using the session-tree capability.
///
/// - `context`: Authorized caller context.
/// - `session`: Stable session identity.
///
/// Unit when the caller may inspect the session's plans.
/// # Returns
/// The authorization result.
/// # Errors
/// Returns an authorization construction or permission error.
pub(crate) fn authorize_plan_read(
    context: &ApplicationContext,
    session: scriberig_core::EntityId,
) -> Result<(), ApplicationError> {
    authorize_session_read(context, &crate::SessionRequest { session })
}

/// Authorizes one durable session mutation.
///
/// - `context`: Caller capabilities and request context.
/// - `session`: Session identity targeted by the mutation.
///
/// Unit when the caller can author plans or modify the session branch tree.
///
/// Returns permission-denied when no applicable grant exists.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn authorize_session_mutation(
    context: &ApplicationContext,
    session: scriberig_core::EntityId,
) -> Result<(), ApplicationError> {
    let plan_capability =
        crate::CapabilityId::new("agent_plan").map_err(|error| ApplicationError::Internal {
            message: error.to_owned(),
        })?;
    let scope = crate::CapabilityScope::Session(session.to_string());
    if context.capabilities.allows(&plan_capability)
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
        action: "agent_plan".to_owned(),
        scope: format!("session: {session}"),
    })
}

/// Authorizes access to safe model descriptors and curated profiles.
///
/// - `context`: Caller capabilities and exact request context.
///
/// Unit when the caller can configure or start agent work.
///
/// Returns permission-denied when the model-library capability is absent.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn authorize_model_library_read(
    context: &ApplicationContext,
) -> Result<(), ApplicationError> {
    let model_library = crate::CapabilityId::new("agent_model_library").map_err(|error| {
        ApplicationError::Internal {
            message: error.to_owned(),
        }
    })?;
    let start_agent =
        crate::CapabilityId::new("agent_start").map_err(|error| ApplicationError::Internal {
            message: error.to_owned(),
        })?;
    if context.capabilities.allows(&model_library) || context.capabilities.allows(&start_agent) {
        return Ok(());
    }
    Err(ApplicationError::PermissionDenied {
        action: "agent_model_library".to_owned(),
        scope: "model_library".to_owned(),
    })
}

/// Authorizes a model-selection change scoped to one session.
///
/// - `context`: Caller capabilities and exact request context.
/// - `session`: Session whose future invocation selection will change.
///
/// Unit when the caller can configure the session or start agent work there.
///
/// Returns permission-denied when no applicable session capability exists.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn authorize_session_model_selection(
    context: &ApplicationContext,
    session: scriberig_core::EntityId,
) -> Result<(), ApplicationError> {
    let capability = crate::CapabilityId::new("session_model_selection").map_err(|error| {
        ApplicationError::Internal {
            message: error.to_owned(),
        }
    })?;
    let start_agent =
        crate::CapabilityId::new("agent_start").map_err(|error| ApplicationError::Internal {
            message: error.to_owned(),
        })?;
    let scope = crate::CapabilityScope::Session(session.to_string());
    if context.capabilities.allows(&capability)
        || context.capabilities.allows(&start_agent)
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
        action: "session_model_selection".to_owned(),
        scope: format!("session: {session}"),
    })
}
