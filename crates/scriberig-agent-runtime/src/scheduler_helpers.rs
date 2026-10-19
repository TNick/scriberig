//! Contains plan scheduler helper functions.

use super::{EntityId, PlanNode, WorkerClaim};
use scriberig_session::{BranchResult, BranchStatus};

/// Resolves the branch identity captured by a claimed worker.
///
/// - `claimed`: Immutable worker snapshots captured before execution.
/// - `claim`: Stable claim identity returned by the worker batch.
///
/// The branch assigned to the matching claim, or a zero identity on mismatch.
/// # Returns
/// The operation result.
pub(super) fn claimed_branch(
    claimed: &[(
        PlanNode,
        WorkerClaim,
        scriberig_app::ResolvedPlanExecution,
        crate::plan_context::PlanWorkerContext,
    )],
    claim: EntityId,
) -> EntityId {
    claimed
        .iter()
        .find(|(_, worker, _, _)| worker.id == claim)
        .map_or(EntityId::from_bytes([0; 16]), |(_, worker, _, _)| {
            worker.branch
        })
}

/// Builds a bounded durable result for one failed worker branch.
///
/// - `id`: Stable failure result identity.
/// - `branch`: Isolated branch that failed.
/// - `error`: Safe runtime failure text.
///
/// An abandoned branch result that cannot satisfy dependencies.
/// # Returns
/// The durable failure result.
/// # Errors
/// This helper cannot fail.
pub(super) fn failed_worker_result(id: EntityId, branch: EntityId, error: &str) -> BranchResult {
    let (code, summary) = error
        .split_once(": ")
        .filter(|(code, _)| code.starts_with("candidate_merge_"))
        .map_or_else(
            || ("worker_failed", "worker execution failed".to_owned()),
            |(code, message)| (code, message.to_owned()),
        );
    BranchResult {
        id,
        branch,
        status: BranchStatus::Abandoned,
        summary: Some(summary),
        result_kind: Some("worker_failure".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec![format!("failure-code:{code}")],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    }
}
