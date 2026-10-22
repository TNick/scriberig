//! Contains durable plan coordinator helper functions.

use super::{
    ApplicationError, Branch, BranchResult, EntityId, ExecutionActivity, Plan, PlanStatus, Session,
};
use crate::plan_context::worker_parent_branch;

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
        let parent_id = worker_parent_branch(plan, node);
        if !session.branches.iter().any(|branch| branch.id == parent_id) {
            return Err("plan worker parent branch is missing".to_owned());
        }
        if let Some(existing) = session
            .branches
            .iter()
            .find(|branch| branch.id == claim.branch)
        {
            if existing.parent != Some(parent_id) {
                return Err("claimed worker branch has a different persisted parent".to_owned());
            }
        } else {
            let mut branch = Branch::new_child(
                claim.branch,
                parent_id,
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
        }
        let parent = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == parent_id)
            .ok_or_else(|| "plan worker parent branch is missing".to_owned())?;
        if !parent.children.contains(&claim.branch) {
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
        let parent = worker_parent_branch(plan, node);
        if let Some(branch) = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == result.branch)
        {
            branch.result = Some(result.clone());
            branch.status = result.status;
            if !branch.activity.iter().any(|activity| {
                matches!(activity, ExecutionActivity::BranchResult(id) if *id == result.id)
            }) {
                branch
                    .activity
                    .push(ExecutionActivity::BranchResult(result.id));
            }
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
///
/// - `plan`: Plan whose coordinator revision changed.
///
/// Conflict error describing the stale coordinator state.
///
/// # Returns
/// An application conflict error for the changed plan revision.
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
///
/// - `message`: Description of the impossible state.
///
/// Internal application error.
///
/// # Returns
/// An internal application error containing the supplied message.
pub(super) fn internal_error(message: impl Into<String>) -> ApplicationError {
    ApplicationError::Internal {
        message: message.into(),
    }
}
#[cfg(test)]
mod tests {
    //! Verifies durable plan results are included in worker branch activity.

    use super::{ensure_worker_branches, persist_worker_results};
    use scriberig_core::EntityId;
    use scriberig_session::{
        Branch, BranchResult, BranchStatus, ExecutionActivity, Plan, PlanNode, PlanNodeParent,
        PlanNodeStatus, Session, WorkerClaim,
    };

    /// Rejects a claimed branch whose persisted parent disagrees with the plan.
    #[test]
    fn claimed_worker_branch_cannot_change_its_persisted_parent() {
        let session_id = EntityId::from_bytes([1; 16]);
        let root_id = EntityId::from_bytes([2; 16]);
        let worker_id = EntityId::from_bytes([3; 16]);
        let wrong_parent_id = EntityId::from_bytes([4; 16]);
        let plan_id = EntityId::from_bytes([5; 16]);
        let node_id = EntityId::from_bytes([6; 16]);
        let root = Branch::new(root_id, "root").expect("root branch");
        let wrong_parent = Branch::new(wrong_parent_id, "wrong parent").expect("parent branch");
        let worker =
            Branch::new_child(worker_id, wrong_parent_id, "worker", "work").expect("worker branch");
        let mut plan = Plan::new(
            plan_id,
            session_id,
            scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
            "complete work",
        )
        .expect("plan");
        plan.nodes.push(PlanNode {
            id: node_id,
            parent: Some(PlanNodeParent::Branch(root_id)),
            title: "worker".to_owned(),
            purpose: "work".to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: PlanNodeStatus::Running,
            branch: Some(worker_id),
            result: None,
            superseded_by: None,
            abandon_reason: None,
            last_failure: None,
            assignment: None,
            claim: Some(WorkerClaim {
                id: EntityId::from_bytes([7; 16]),
                node: node_id,
                branch: worker_id,
                owner: "coordinator".to_owned(),
                generation: 1,
                assignment: None,
            }),
            claim_generation: 1,
        });
        let mut session = Session {
            id: session_id,
            branches: vec![root, wrong_parent, worker],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        };

        assert_eq!(
            ensure_worker_branches(&mut session, &plan),
            Err("claimed worker branch has a different persisted parent".to_owned())
        );
    }

    /// Verifies an existing worker branch receives one idempotent result event.
    #[test]
    fn existing_worker_branch_records_result_activity_once() {
        let session_id = EntityId::from_bytes([1; 16]);
        let root_branch = EntityId::from_bytes([2; 16]);
        let worker_branch = EntityId::from_bytes([3; 16]);
        let plan_id = EntityId::from_bytes([4; 16]);
        let node_id = EntityId::from_bytes([5; 16]);
        let result_id = EntityId::from_bytes([6; 16]);
        let root = Branch::new(root_branch, "root").expect("root branch");
        let worker =
            Branch::new_child(worker_branch, root_branch, "worker", "work").expect("worker branch");
        let mut plan = Plan::new(
            plan_id,
            session_id,
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: root_branch,
            },
            "complete work",
        )
        .expect("plan");
        plan.nodes.push(PlanNode {
            id: node_id,
            parent: Some(PlanNodeParent::Branch(root_branch)),
            title: "worker".to_owned(),
            purpose: "work".to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: PlanNodeStatus::Completed,
            branch: Some(worker_branch),
            result: Some(result_id),
            superseded_by: None,
            abandon_reason: None,
            last_failure: None,
            assignment: None,
            claim: None,
            claim_generation: 1,
        });
        let result = BranchResult {
            id: result_id,
            branch: worker_branch,
            status: BranchStatus::Completed,
            summary: Some("finished".to_owned()),
            result_kind: None,
            candidate_context: None,
            references: Vec::new(),
            result_handles: Vec::new(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        };
        let mut session = Session {
            id: session_id,
            branches: vec![root, worker],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        };

        persist_worker_results(&mut session, &plan, std::slice::from_ref(&result))
            .expect("persist result");
        persist_worker_results(&mut session, &plan, &[result]).expect("persist repeated result");

        let worker = session
            .branches
            .iter()
            .find(|branch| branch.id == worker_branch)
            .expect("worker branch remains present");
        assert_eq!(
            worker.result.as_ref().map(|value| value.id),
            Some(result_id)
        );
        assert_eq!(
            worker
                .activity
                .iter()
                .filter(|activity| matches!(activity, ExecutionActivity::BranchResult(id) if *id == result_id))
                .count(),
            1
        );
    }
}
