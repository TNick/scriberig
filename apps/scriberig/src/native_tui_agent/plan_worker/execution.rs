//! Executes plan workers through upstream or retained local providers.

use std::sync::Arc;

use rpi_ai::{Model, Provider, RequestSecret};
use scriberig_agent_runtime::scheduler::PlanWorkerExecutionResult;

use super::results::{completed_worker_result, failed_plan_worker_result};
use super::{NativePlanWorkerExecutor, PlanNode, WorkerClaim};

impl<B> NativePlanWorkerExecutor<'_, B>
where
    B: super::ApplicationBackend + Sync,
{
    /// Attempts the upstream provider path when production composition exists.
    ///
    /// - `node`: Exact durable work node selected by the coordinator.
    /// - `claim`: Exclusive branch claim persisted before provider execution.
    /// - `execution`: Resolved provider, model, profile, and thinking settings.
    /// - `worker_context`: Exact compiled branch context.
    /// - `cancelled`: Shared cancellation flag for this plan operation.
    /// - `connection`: Safe model-library connection identity.
    ///
    /// # Returns
    /// The upstream worker result when an upstream runner and provider exist.
    ///
    /// # Errors
    /// Returns an error when upstream provider resolution or execution fails.
    pub(super) fn execute_upstream_worker(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
        worker_context: &scriberig_agent_runtime::plan_context::PlanWorkerContext,
        cancelled: &std::sync::atomic::AtomicBool,
        connection: &str,
    ) -> Result<Option<PlanWorkerExecutionResult>, String> {
        let Some(runner) = self.runner else {
            return Ok(None);
        };
        let upstream = runner
            .upstream_provider_for(&execution.model, connection)
            .map_err(|error| error.to_string())?;
        let Some((upstream_provider, model, request_secret)) = upstream else {
            return Ok(None);
        };
        self.execute_bound_upstream_worker(
            node,
            claim,
            execution,
            worker_context,
            cancelled,
            upstream_provider,
            model,
            request_secret,
        )
        .map(Some)
    }

    /// Executes one branch using an already authorized upstream provider.
    ///
    /// # Parameters
    /// - `node`: Exact durable work node selected by the coordinator.
    /// - `claim`: Exclusive branch claim persisted before provider execution.
    /// - `execution`: Resolved provider, model, profile, and thinking settings.
    /// - `worker_context`: Exact compiled branch context.
    /// - `cancelled`: Shared cancellation flag for this plan operation.
    /// - `provider`: Upstream provider authorized for the exact model.
    /// - `upstream_provider`: Upstream provider implementation for the resolved model.
    /// - `model`: Exact model retained by the resolved execution.
    /// - `request_secret`: Optional protected request credential.
    ///
    /// # Returns
    /// The completed or failed worker result with captured evidence.
    ///
    /// # Errors
    /// Returns an execution failure without durable evidence.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_bound_upstream_worker(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
        worker_context: &scriberig_agent_runtime::plan_context::PlanWorkerContext,
        cancelled: &std::sync::atomic::AtomicBool,
        upstream_provider: Arc<dyn Provider>,
        model: Model,
        request_secret: Option<RequestSecret>,
    ) -> Result<PlanWorkerExecutionResult, String> {
        let invocation = scriberig_agent_runtime::upstream_invocation::UpstreamBranchInvocation {
            branch: claim.branch,
            provider: upstream_provider,
            model,
            request_secret,
            bearer: true,
            applied_settings: None,
            provider_profile: None,
            thinking_level: execution.thinking_level.unwrap_or_default(),
            context: worker_context.compiled.clone(),
            tools: Vec::new(),
        };
        let outcome = super::super::upstream_execution::execute_precompiled_branch(
            self.service,
            self.context,
            self.session,
            claim.branch,
            &node.purpose,
            Arc::clone(&self.project_store),
            invocation,
            worker_context.inputs.project_scope.clone(),
            cancelled,
        );
        outcome
            .map(|result| {
                let agent_result = scriberig_agent_runtime::AgentResult {
                    output: result.result.output,
                    tool_evidence: result.tool_evidence,
                    model_requests: result.result.model_requests,
                };
                completed_worker_result(claim.branch, execution, agent_result)
            })
            .or_else(|failure| {
                if failure.failure.model_requests.is_empty() && failure.tool_evidence.is_empty() {
                    Err(failure.failure.error.to_string())
                } else {
                    Ok(PlanWorkerExecutionResult {
                        result: failed_plan_worker_result(claim.branch),
                        model_requests: failure.failure.model_requests,
                        tool_evidence: failure.tool_evidence,
                    })
                }
            })
    }
}
