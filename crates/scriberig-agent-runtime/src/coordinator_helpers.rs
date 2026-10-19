//! Contains durable plan coordinator helper functions.

use super::{
    ApplicationError, Branch, BranchResult, EntityId, ExecutionActivity, Plan, PlanNodeParent,
    PlanStatus, Session,
};

/// Returns all durable branch results in a session snapshot.
/// # Parameters
/// - `session`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn branch_results(session: &Session) -> Vec<BranchResult> {
    session
        .branches
        .iter()
        .filter_map(|branch| branch.result.clone())
        .collect()
}

/// Finds one durable plan by exact identity.
/// # Parameters
/// - `session`: Input for this operation.
/// - `plan_id`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn get_plan(session: &Session, plan_id: EntityId) -> Result<&Plan, ApplicationError> {
    session
        .plans
        .iter()
        .find(|plan| plan.id == plan_id)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: plan_id.to_string(),
        })
}

/// Ensures this runtime still owns the durable coordinator claim.
/// # Parameters
/// - `plan`: Input for this operation.
/// - `claim`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn require_coordinator(plan: &Plan, claim: EntityId) -> Result<(), ApplicationError> {
    if plan.status == PlanStatus::Running
        && plan.coordinator_claim.as_ref().map(|active| active.id) == Some(claim)
    {
        Ok(())
    } else {
        Err(revision_conflict(plan.id))
    }
}

/// Replaces one plan in a session projection without changing other plans.
/// # Parameters
/// - `session`: Input for this operation.
/// - `plan`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn replace_plan(session: &mut Session, plan: Plan) -> Result<(), ApplicationError> {
    let target = session
        .plans
        .iter_mut()
        .find(|target| target.id == plan.id)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: plan.id.to_string(),
        })?;
    *target = plan;
    Ok(())
}

/// Adds newly claimed worker branches to the proposed plan snapshot.
/// # Parameters
/// - `session`: Input for this operation.
/// - `plan`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn ensure_worker_branches(session: &mut Session, plan: &Plan) -> Result<(), String> {
    for node in &plan.nodes {
        let Some(claim) = &node.claim else {
            continue;
        };
        if session
            .branches
            .iter()
            .any(|branch| branch.id == claim.branch)
        {
            continue;
        }
        let parent = match node.parent {
            Some(PlanNodeParent::Branch(branch)) => branch,
            Some(PlanNodeParent::Node(parent_node)) => plan
                .nodes
                .iter()
                .find(|candidate| candidate.id == parent_node)
                .and_then(|candidate| candidate.branch)
                .unwrap_or(plan.root_branch),
            None => plan.root_branch,
        };
        let mut branch = Branch::new_child(
            claim.branch,
            parent,
            node.title.clone(),
            node.purpose.clone(),
        )
        .map_err(|error| format!("worker branch invalid: {error}"))?;
        if let Some(anchor) = plan.anchor {
            branch.continuation = Some(scriberig_session::ContinuationProvenance {
                anchor,
                origin: scriberig_session::ContinuationOrigin::PlanExecution,
            });
        }
        session.branches.push(branch);
        if let Some(parent) = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == parent)
            && !parent.children.contains(&claim.branch)
        {
            parent.children.push(claim.branch);
        }
    }
    Ok(())
}

/// Persists worker results on their exact isolated branches.
/// # Parameters
/// - `session`: Input for this operation.
/// - `plan`: Input for this operation.
/// - `results`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn persist_worker_results(
    session: &mut Session,
    plan: &Plan,
    results: &[BranchResult],
) -> Result<(), ApplicationError> {
    for result in results {
        let node = plan
            .nodes
            .iter()
            .find(|node| node.branch == Some(result.branch))
            .ok_or_else(|| internal_error("worker result branch is not claimed by the plan"))?;
        let parent = match node.parent {
            Some(PlanNodeParent::Branch(branch)) => branch,
            Some(PlanNodeParent::Node(parent_node)) => plan
                .nodes
                .iter()
                .find(|candidate| candidate.id == parent_node)
                .and_then(|candidate| candidate.branch)
                .unwrap_or(plan.root_branch),
            None => plan.root_branch,
        };
        if let Some(branch) = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == result.branch)
        {
            branch.result = Some(result.clone());
            branch.status = result.status;
        } else {
            let mut branch = Branch::new_child(
                result.branch,
                parent,
                node.title.clone(),
                node.purpose.clone(),
            )
            .map_err(|error| internal_error(format!("worker branch invalid: {error}")))?;
            branch.status = result.status;
            branch.result = Some(result.clone());
            branch
                .activity
                .push(ExecutionActivity::BranchResult(result.id));
            session.branches.push(branch);
            if let Some(parent) = session
                .branches
                .iter_mut()
                .find(|branch| branch.id == parent)
            {
                if !parent.children.contains(&result.branch) {
                    parent.children.push(result.branch);
                }
            }
        }
    }
    Ok(())
}

/// Creates a stable application error for coordinator revision races.
/// # Parameters
/// - `plan`: Input for this operation.
/// # Returns
/// The operation result.
pub(super) fn revision_conflict(plan: EntityId) -> ApplicationError {
    ApplicationError::Conflict {
        subject: plan.to_string(),
        details: Box::new(scriberig_app::ConflictDetails {
            operation: Some("plan coordinator revision".to_owned()),
            ..scriberig_app::ConflictDetails::default()
        }),
    }
}

/// Creates an application error for impossible scheduler state.
/// # Parameters
/// - `message`: Input for this operation.
/// # Returns
/// The operation result.
pub(super) fn internal_error(message: impl Into<String>) -> ApplicationError {
    ApplicationError::Internal {
        message: message.into(),
    }
}
