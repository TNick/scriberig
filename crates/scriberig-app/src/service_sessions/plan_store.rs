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
    let previous_plan = stored_plan.clone();
    let settled_results = proposed_plan.settlement_result_refs_since(&previous_plan);
    if proposed_plan.revision != expected_revision.saturating_add(1) {
        return Err(ApplicationError::Internal {
            message: "proposed plan has an unexpected revision".to_owned(),
        });
    }
    let settled_updates = settled_results
        .into_iter()
        .map(|(branch_id, result_id)| {
            let branch = session
                .branches
                .iter()
                .find(|branch| branch.id == branch_id)
                .ok_or_else(|| ApplicationError::Internal {
                    message: "settled worker branch is absent".to_owned(),
                })?;
            if branch.result.as_ref().map(|result| result.id) != Some(result_id)
                || branch.result.as_ref().map(|result| result.branch) != Some(branch_id)
            {
                return Err(ApplicationError::Internal {
                    message: "settled worker result does not match its plan".to_owned(),
                });
            }
            Ok(branch.clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    *stored_plan = proposed_plan.clone();
    for branch in &session.branches {
        if stored.branches.iter().any(|known| known.id == branch.id) {
            continue;
        }
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
    for proposed_branch in settled_updates {
        if let Some(stored_branch) = stored
            .branches
            .iter_mut()
            .find(|branch| branch.id == proposed_branch.id)
        {
            *stored_branch = proposed_branch;
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
