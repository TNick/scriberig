//! Builds and integrates durable plan-worker results.

use std::sync::Arc;

use scriberig_agent_runtime::scheduler::PlanWorkerExecutionResult;
use scriberig_core::EntityId;
use scriberig_session::{BranchResult, BranchStatus, WorkerClaim};
use scriberig_store_sqlite::SqliteStore;

use crate::child_runtime::compact_result_handles;

/// Builds the durable result for a completed model-backed plan worker.
///
/// - `branch`: Exact worker branch that produced the result.
/// - `execution`: Resolved provider, model, and thinking assignment.
/// - `result`: Runtime output and captured model request projections.
///
/// # Returns
/// A completed branch result with safe model provenance handles.
///
/// # Errors
/// This helper is infallible; `PlanWorkerExecutionResult` is a value type.
pub(super) fn completed_worker_result(
    branch: EntityId,
    execution: &scriberig_app::ResolvedPlanExecution,
    result: scriberig_agent_runtime::AgentResult,
) -> PlanWorkerExecutionResult {
    let mut result_handles = compact_result_handles(&result);
    result_handles.push(format!("provider:{}", execution.provider_id));
    result_handles.push(format!("model:{}", execution.model_id));
    if let Some(thinking) = execution.thinking_level {
        result_handles.push(format!("thinking:{}", thinking.as_str()));
    }
    PlanWorkerExecutionResult {
        result: BranchResult {
            id: EntityId::new(),
            branch,
            status: BranchStatus::Completed,
            summary: Some(result.output),
            result_kind: Some("plan-worker".to_owned()),
            candidate_context: None,
            references: Vec::new(),
            result_handles,
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        },
        model_requests: result.model_requests,
        tool_evidence: result.tool_evidence,
    }
}

/// Creates the bounded failed result used when captured requests precede error.
///
/// - `branch`: Exact claimed branch that executed the failed request.
///
/// # Returns
/// An abandoned worker result that cannot satisfy plan dependencies.
///
/// # Errors
/// This helper is infallible; failure is represented in the returned result.
pub(super) fn failed_plan_worker_result(branch: EntityId) -> BranchResult {
    BranchResult {
        id: EntityId::new(),
        branch,
        status: BranchStatus::Abandoned,
        summary: Some("worker execution failed".to_owned()),
        result_kind: Some("worker_failure".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec!["failure-code:worker_failed".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    }
}

/// Merges the exact candidate handles admitted from fulfilled dependencies.
///
/// This operation creates a reviewable candidate at the shared base snapshot;
/// validation and explicit project application remain separate operations.
///
/// - `project_store`: Durable project and candidate owner.
/// - `claim`: Exclusive worker branch claim.
/// - `worker_context`: Exact dependency results compiled for this node.
///
/// # Returns
/// A completed branch result containing the newly merged candidate handle.
///
/// # Errors
/// Returns an error when merge inputs are invalid or conflict.
pub(super) fn integrate_dependency_candidates(
    project_store: &Arc<SqliteStore>,
    claim: &WorkerClaim,
    worker_context: &scriberig_agent_runtime::plan_context::PlanWorkerContext,
) -> Result<BranchResult, String> {
    let candidates = worker_context
        .inputs
        .dependency_results
        .iter()
        .flat_map(|result| &result.semantic_handles)
        .filter_map(|handle| handle.strip_prefix("candidate:"))
        .collect::<Vec<_>>();
    let [left, right] = candidates.as_slice() else {
        return Err(format!(
            "candidate_merge_invalid: integration node requires exactly two dependency candidate handles; received {}",
            candidates.len()
        ));
    };
    let merged = scriberig_project::ProjectService::new(Arc::clone(project_store))
        .merge_project_candidates(left, right)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let summary = format!(
        "integrated candidate {} combines {} and {} at exact base {}",
        merged.id, left, right, merged.base_snapshot
    );
    Ok(BranchResult {
        id: EntityId::new(),
        branch: claim.branch,
        status: BranchStatus::Completed,
        summary: Some(summary),
        result_kind: Some("project_candidate_merge".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec![
            format!("candidate:{}", merged.id),
            format!("candidate_parent:{left}"),
            format!("candidate_parent:{right}"),
            format!("project:{}", merged.project),
            format!("snapshot:{}", merged.base_snapshot),
        ],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    })
}
