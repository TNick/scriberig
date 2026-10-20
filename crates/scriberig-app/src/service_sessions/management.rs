//! Lists and removes sessions through the application-owned session boundary.

use super::ApplicationService;
use crate::{ApplicationBackend, ApplicationContext, ApplicationError};
use scriberig_core::EntityId;
use scriberig_session::{BranchStatus, PlanNodeStatus, PlanStatus, Session};

/// Lists durable sessions after checking the session-management capability.
///
/// - `service`: Application service owning the session repository.
/// - `context`: Authorized caller context.
///
/// Complete sessions in stable identity order.
///
/// Returns authorization, repository, or process-lock failures.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn list_sessions<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
) -> Result<Vec<Session>, ApplicationError> {
    authorize_session_management(context)?;
    let mut sessions = if let Some(store) = &service.session_store {
        store
            .list()
            .map_err(|message| ApplicationError::Internal { message })?
    } else {
        Vec::new()
    };
    let local = service
        .sessions
        .lock()
        .map_err(|_| ApplicationError::Internal {
            message: "application session lock failed".to_owned(),
        })?;
    for session in local.values() {
        if !sessions.iter().any(|stored| stored.id == session.id) {
            sessions.push(session.clone());
        }
    }
    sessions.sort_by_key(|session| session.id);
    Ok(sessions)
}

/// Removes one idle session after validating the exact identity.
///
/// Session records with active branches or coordinators remain available until
/// their work is complete or explicitly stopped.
///
/// - `service`: Application service owning the session repository.
/// - `context`: Authorized caller context.
/// - `session_id`: Stable identity of the session to remove.
///
/// Unit after the durable and process-local record is removed.
///
/// Returns authorization, lookup, active-work, or persistence failures.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when removal is rejected or fails.
pub(super) fn remove_session<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session_id: EntityId,
) -> Result<(), ApplicationError> {
    authorize_session_management(context)?;
    let session =
        service
            .load_session_state(session_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                subject: format!("session {session_id}"),
            })?;
    if session
        .branches
        .iter()
        .any(|branch| branch.status == BranchStatus::Active)
        || session.plans.iter().any(|plan| {
            plan.status == PlanStatus::Running
                || plan
                    .nodes
                    .iter()
                    .any(|node| node.status == PlanNodeStatus::Running)
        })
    {
        return Err(ApplicationError::Conflict {
            subject: format!("session {session_id} has active work"),
            details: Box::new(crate::ConflictDetails {
                operation: Some("session removal requires idle work".to_owned()),
                ..crate::ConflictDetails::default()
            }),
        });
    }
    if let Some(store) = &service.session_store {
        store
            .remove(session_id)
            .map_err(|message| ApplicationError::Internal { message })?;
    }
    service
        .sessions
        .lock()
        .map_err(|_| ApplicationError::Internal {
            message: "application session lock failed".to_owned(),
        })?
        .remove(&session_id);
    Ok(())
}

/// Checks the explicit capability required to enumerate or remove sessions.
///
/// - `context`: Caller capability set.
///
/// Unit when session management is granted.
///
/// Returns permission-denied when the capability is absent.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the caller is unauthorized.
pub(super) fn authorize_session_management(
    context: &ApplicationContext,
) -> Result<(), ApplicationError> {
    let capability = crate::CapabilityId::new("session_management").map_err(|error| {
        ApplicationError::Internal {
            message: error.to_owned(),
        }
    })?;
    if context.capabilities.allows(&capability) {
        return Ok(());
    }
    Err(ApplicationError::PermissionDenied {
        action: "session_management".to_owned(),
        scope: "sessions".to_owned(),
    })
}
