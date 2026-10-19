//! Calculates provider-independent plan dispatch decisions.

use scriberig_core::EntityId;
use scriberig_session::{BranchResult, Plan, PlanNode, PlanNodeStatus, PlanStatus, WorkerClaim};

#[path = "scheduler_execution.rs"]
mod execution;
#[path = "scheduler_execution_helpers.rs"]
mod execution_helpers;

#[path = "scheduler_results.rs"]
mod results;
pub use results::WorkerBatchResult;
#[path = "scheduler_helpers.rs"]
mod scheduler_helpers;

/// One deterministic view of plan work and eligible dispatches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleTick {
    /// Whether the plan has reached a terminal lifecycle state.
    pub terminal: bool,
    /// Whether automatic dispatch is paused.
    pub paused: bool,
    /// Whether a coordinator currently owns the plan.
    pub coordinator_owned: bool,
    /// Stable identities of currently executing worker nodes.
    pub running: Vec<EntityId>,
    /// Stable identities blocked by missing dependency results.
    pub blocked: Vec<EntityId>,
    /// Stable identities in failure state.
    pub failed: Vec<EntityId>,
    /// Ready node identities selected within the concurrency budget.
    pub dispatch: Vec<EntityId>,
}

/// Computes deterministic plan scheduling decisions without provider calls.
pub struct PlanScheduler;

/// Executes one already claimed plan worker branch.
pub trait PlanWorkerExecutor: Sync {
    /// Runs one isolated node with its durable worker claim.
    ///
    /// - `node`: Exact node snapshot selected for this invocation.
    /// - `claim`: Exclusive branch claim captured before provider invocation.
    ///
    /// The durable branch result produced by the worker.
    ///
    /// Returns a safe worker failure summary.
    /// # Parameters
    /// - `execution`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String>;

    /// Runs one claimed worker with its exact compiled semantic context.
    ///
    /// - `node`: Exact durable node snapshot.
    /// - `claim`: Exclusive branch claim captured before provider invocation.
    /// - `execution`: Resolved model/provider/thinking assignment.
    /// - `context`: Exact anchor, semantic scope, ancestor, and dependency inputs.
    ///
    /// The durable branch result produced by the worker.
    ///
    /// Returns a safe worker failure summary.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute_with_context(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
        context: &crate::plan_context::PlanWorkerContext,
    ) -> Result<BranchResult, String> {
        let _ = context;
        self.execute(node, claim, execution)
    }
}

impl PlanScheduler {
    /// Computes one dispatch tick from an immutable plan snapshot.
    ///
    /// The eligibility predicate represents trusted model assignment
    /// resolution performed before dispatch. The scheduler never invokes a
    /// provider or starts a branch.
    ///
    /// - `plan`: Durable plan snapshot.
    /// - `results`: Exact durable branch results available to dependency checks.
    /// - `concurrency_budget`: Maximum total workers allowed for the plan.
    /// - `assignment_is_eligible`: Trusted predicate for resolved assignments.
    ///
    /// A stable plan summary and up to the available number of dispatches.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn tick(
        plan: &Plan,
        results: &[BranchResult],
        concurrency_budget: usize,
        assignment_is_eligible: impl Fn(&scriberig_session::PlanNode) -> bool,
    ) -> ScheduleTick {
        let mut running = plan
            .nodes
            .iter()
            .filter(|node| node.status == PlanNodeStatus::Running)
            .map(|node| node.id)
            .collect::<Vec<_>>();
        running.sort();
        let mut failed = plan
            .nodes
            .iter()
            .filter(|node| node.status == PlanNodeStatus::Failed)
            .map(|node| node.id)
            .collect::<Vec<_>>();
        failed.sort();
        let ready = plan.ready_nodes_from_results(results);
        let blocked = plan
            .nodes
            .iter()
            .filter(|node| {
                matches!(node.status, PlanNodeStatus::Planned | PlanNodeStatus::Ready)
                    && !ready.contains(&node.id)
            })
            .map(|node| node.id)
            .collect::<Vec<_>>();
        let terminal = matches!(
            plan.status,
            PlanStatus::Completed | PlanStatus::Failed | PlanStatus::Cancelled
        );
        let paused = plan.status == PlanStatus::Paused;
        let coordinator_owned = plan.coordinator_claim.is_some();
        let capacity = concurrency_budget.saturating_sub(running.len());
        let dispatch = if plan.status == PlanStatus::Running && coordinator_owned {
            plan.nodes
                .iter()
                .filter(|node| ready.contains(&node.id) && assignment_is_eligible(node))
                .take(capacity)
                .map(|node| node.id)
                .collect()
        } else {
            Vec::new()
        };

        ScheduleTick {
            terminal,
            paused,
            coordinator_owned,
            running,
            blocked,
            failed,
            dispatch,
        }
    }
}
