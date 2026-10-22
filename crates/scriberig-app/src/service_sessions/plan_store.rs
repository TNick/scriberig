//! Persists application-owned plan revisions with revision checks.

use super::{ApplicationBackend, ApplicationService};
use crate::{ApplicationContext, ApplicationError};
use scriberig_core::EntityId;
use scriberig_session::Session;

/// Applies a plan revision through the durable, backend, or local store.
///
/// - `service`: Application service owning the plan and session stores.
/// - `context`: Authorized caller context.
/// - `session`: Complete session snapshot carrying the updated plan.
/// - `plan`: Stable plan identity.
/// - `expected_revision`: Revision observed before the mutation.
///
/// Unit after the proposed plan revision is persisted.
/// # Returns
/// The operation result.
/// # Errors
/// Returns authorization, conflict, validation, or persistence failures.
pub(super) fn compare_and_swap_plan<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: &Session,
    plan: EntityId,
    expected_revision: u64,
) -> Result<(), ApplicationError> {
    super::service_session_authorization::authorize_session_mutation(context, session.id)?;
    if let Some(store) = &service.session_store {
        return store
            .compare_and_swap_plan(session, plan, expected_revision)
            .map_err(map_plan_store_error);
    }
    match service
        .backend
        .compare_and_swap_plan(context, session, plan, expected_revision)
    {
        Ok(()) => return Ok(()),
        Err(error) if error.category() != "unsupported_operation" => return Err(error),
        Err(_) => {}
    }
    let mut sessions = service
        .sessions
        .lock()
        .map_err(|_| ApplicationError::Internal {
            message: "application session lock failed".to_owned(),
        })?;
    let stored = sessions
        .get_mut(&session.id)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: session.id.to_string(),
        })?;
    let proposed_plan = session
        .plans
        .iter()
        .find(|entry| entry.id == plan)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: plan.to_string(),
        })?;
    let settled_branches = proposed_plan
        .nodes
        .iter()
        .filter_map(|node| node.branch)
        .collect::<std::collections::BTreeSet<_>>();
    let stored_plan = stored
        .plans
        .iter_mut()
        .find(|entry| entry.id == plan)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: plan.to_string(),
        })?;
    if stored_plan.revision != expected_revision {
        return Err(map_plan_store_error(
            scriberig_session::PlanStoreError::RevisionConflict {
                expected: expected_revision,
                actual: stored_plan.revision,
            },
        ));
    }
    if proposed_plan.revision != expected_revision.saturating_add(1) {
        return Err(ApplicationError::Internal {
            message: "proposed plan has an unexpected revision".to_owned(),
        });
    }
    *stored_plan = proposed_plan.clone();
    for branch in &session.branches {
        if let Some(known) = stored
            .branches
            .iter_mut()
            .find(|known| known.id == branch.id)
        {
            if settled_branches.contains(&branch.id) && branch.result.is_some() {
                *known = branch.clone();
            }
        } else {
            stored.branches.push(branch.clone());
            if let Some(parent_id) = branch.parent
                && let Some(parent) = stored
                    .branches
                    .iter_mut()
                    .find(|known| known.id == parent_id)
                && !parent.children.contains(&branch.id)
            {
                parent.children.push(branch.id);
            }
        }
    }
    Ok(())
}

/// Converts a plan repository error into a stable application failure.
///
/// - `error`: Typed repository failure.
///
/// A conflict or internal application failure.
/// # Returns
/// The mapped application error.
fn map_plan_store_error(error: scriberig_session::PlanStoreError) -> ApplicationError {
    match error {
        scriberig_session::PlanStoreError::RevisionConflict { expected, actual } => {
            ApplicationError::Conflict {
                subject: "plan revision".to_owned(),
                details: Box::new(crate::ConflictDetails {
                    operation: Some(format!("expected {expected}, actual {actual}")),
                    ..crate::ConflictDetails::default()
                }),
            }
        }
        scriberig_session::PlanStoreError::Persistence(message) => {
            ApplicationError::Internal { message }
        }
    }
}
