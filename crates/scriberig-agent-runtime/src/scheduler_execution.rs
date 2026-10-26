//! Executes plan scheduler worker dispatches.

use super::{PlanScheduler, PlanWorkerExecutor, WorkerBatchResult, execution_helpers};
use scriberig_core::EntityId;
use scriberig_session::{BranchResult, Plan, PlanNode, WorkerClaim};

impl PlanScheduler {
    /// Claims and runs one dispatch set concurrently, then links its results.
    ///
    /// Model selection is resolved for every dispatch before claims or provider
    /// execution begin. The caller owns persistence of returned state.
    ///
    /// - `plan`: Mutable authoritative plan projection.
    /// - `dispatch`: Stable nodes selected by one scheduler tick.
    /// - `results`: Existing durable results used for readiness checks.
    /// - `executor`: Shared worker factory/provider boundary.
    /// - `upstream_models`: Host-owned upstream model catalog.
    /// - `selection`: Session-level model defaults.
    /// - `runtime_fallback`: Runtime model used when no assignment is selected.
    /// - `next_id`: Allocates branch and claim identities.
    ///
    /// Completed and failed nodes with result records for persistence.
    ///
    /// Returns an error before executor invocation when assignment resolution or
    /// exclusive claim acquisition fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_dispatch(
        plan: &mut Plan,
        dispatch: &[EntityId],
        results: &[BranchResult],
        executor: &impl PlanWorkerExecutor,
        upstream_models: &[rpi_ai::Model],
        selection: &scriberig_session::SessionExecutionSelection,
        runtime_fallback: &str,
        next_id: impl FnMut() -> EntityId,
    ) -> Result<WorkerBatchResult, String> {
        Self::execute_dispatch_persisted(
            plan,
            dispatch,
            results,
            executor,
            upstream_models,
            selection,
            runtime_fallback,
            next_id,
            |_| Ok(()),
        )
    }

    /// Persists exclusive worker claims before invoking any worker provider.
    ///
    /// - `plan`: Mutable authoritative plan projection.
    /// - `dispatch`: Stable nodes selected by one scheduler tick.
    /// - `results`: Existing durable results used for readiness checks.
    /// - `executor`: Shared worker factory/provider boundary.
    /// - `upstream_models`: Host-owned upstream model catalog.
    /// - `selection`: Session-level model defaults.
    /// - `runtime_fallback`: Runtime model used when no assignment is selected.
    /// - `next_id`: Allocates branch and claim identities.
    /// - `persist_claims`: Durably records running states before worker calls.
    ///
    /// Completed and failed nodes with result records for persistence.
    ///
    /// Returns before provider invocation when resolution, claim acquisition,
    /// or durable claim persistence fails.
    #[allow(clippy::too_many_arguments)]
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn execute_dispatch_persisted(
        plan: &mut Plan,
        dispatch: &[EntityId],
        results: &[BranchResult],
        executor: &impl PlanWorkerExecutor,
        upstream_models: &[rpi_ai::Model],
        selection: &scriberig_session::SessionExecutionSelection,
        runtime_fallback: &str,
        next_id: impl FnMut() -> EntityId,
        persist_claims: impl FnOnce(&Plan) -> Result<(), String>,
    ) -> Result<WorkerBatchResult, String> {
        Self::execute_dispatch_with_context_persisted(
            plan,
            dispatch,
            results,
            executor,
            upstream_models,
            selection,
            runtime_fallback,
            next_id,
            |plan, node, claim| {
                Ok(crate::plan_context::PlanWorkerContext::minimal(
                    plan, node, claim,
                ))
            },
            persist_claims,
        )
    }

    /// Executes a dispatch with a caller-compiled exact context per claim.
    ///
    /// - `plan`: Mutable authoritative plan projection.
    /// - `dispatch`: Stable nodes selected by one scheduler tick.
    /// - `results`: Existing durable results used for readiness checks.
    /// - `executor`: Shared worker factory/provider boundary.
    /// - `upstream_models`: Host-owned upstream model catalog.
    /// - `selection`: Session-level model defaults.
    /// - `runtime_fallback`: Runtime model used when a node has no assignment.
    /// - `next_id`: Allocates branch and claim identities.
    /// - `compile_context`: Compiles semantic inputs for each claimed worker.
    /// - `persist_claims`: Durably records running states before worker calls.
    ///
    /// Completed and failed nodes with result records for persistence.
    ///
    /// Returns before provider invocation when model/context resolution, claim
    /// acquisition, or durable claim persistence fails.
    #[allow(clippy::too_many_arguments)]
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn execute_dispatch_with_context_persisted(
        plan: &mut Plan,
        dispatch: &[EntityId],
        results: &[BranchResult],
        executor: &impl PlanWorkerExecutor,
        upstream_models: &[rpi_ai::Model],
        selection: &scriberig_session::SessionExecutionSelection,
        runtime_fallback: &str,
        mut next_id: impl FnMut() -> EntityId,
        compile_context: impl Fn(
            &Plan,
            &PlanNode,
            &WorkerClaim,
        ) -> Result<crate::plan_context::PlanWorkerContext, String>,
        persist_claims: impl FnOnce(&Plan) -> Result<(), String>,
    ) -> Result<WorkerBatchResult, String> {
        let resolved = execution_helpers::resolve_assignments(
            plan,
            dispatch,
            upstream_models,
            selection,
            runtime_fallback,
        )?;
        let claimed = execution_helpers::claim_workers(
            plan,
            dispatch,
            results,
            &resolved,
            &mut next_id,
            compile_context,
            persist_claims,
        )?;
        let executions = execution_helpers::execute_workers(&claimed, executor);
        execution_helpers::settle_workers(plan, results, &claimed, executions, next_id)
    }
}
