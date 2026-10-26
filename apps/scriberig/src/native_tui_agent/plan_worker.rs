//! Executes claimed plan branches through the native semantic agent runtime.

#[path = "plan_worker/execution.rs"]
mod execution;
#[path = "plan_worker/results.rs"]
mod results;

#[cfg(test)]
use std::collections::BTreeMap;
use std::sync::Arc;

use scriberig_agent_runtime::scheduler::{PlanWorkerExecutionResult, PlanWorkerExecutor};
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_core::EntityId;
use scriberig_session::{BranchResult, PlanNode, WorkerClaim};
use scriberig_store_sqlite::SqliteStore;

use self::results::integrate_dependency_candidates;
#[cfg(test)]
use crate::native_scenario::NativeScenario;

/// Executes one claimed plan node with its resolved assignment and context.
pub(super) struct NativePlanWorkerExecutor<'a, B> {
    /// Native root composition owning the protected upstream provider factory.
    runner: Option<&'a super::NativeTuiAgentRunner>,
    /// Shared application boundary for semantic tools and durable children.
    service: &'a ApplicationService<B>,
    /// Trusted context inherited from the lifecycle request.
    context: &'a ApplicationContext,
    /// Exact session owning this plan.
    session: EntityId,
    /// Deterministic scenario adapters keyed by safe connection identity.
    #[cfg(test)]
    scenario_providers: &'a BTreeMap<String, NativeScenario>,
    /// Durable project catalog used by semantic candidate operations.
    project_store: Arc<SqliteStore>,
    /// Shared cancellable coordinator operation.
    operation: Option<scriberig_app::OperationId>,
}

impl<'a, B> NativePlanWorkerExecutor<'a, B> {
    /// Creates a test executor using the upstream loop and scenario adapters.
    ///
    /// # Parameters
    /// - `service`: Shared application boundary for semantic tools.
    /// - `context`: Trusted caller capabilities and scope.
    /// - `session`: Exact session owning the plan.
    /// - `scenario_providers`: Deterministic adapters for test scenarios.
    /// - `project_store`: Durable project catalog and candidate store.
    /// - `operation`: Shared coordinator operation used for cancellation.
    ///
    /// # Returns
    /// An executor that still runs every provider turn through `rpi-agent`.
    #[cfg(test)]
    pub(super) fn new(
        service: &'a ApplicationService<B>,
        context: &'a ApplicationContext,
        session: EntityId,
        scenario_providers: &'a BTreeMap<String, NativeScenario>,
        project_store: Arc<SqliteStore>,
        operation: Option<scriberig_app::OperationId>,
    ) -> Self {
        Self {
            runner: None,
            service,
            context,
            session,
            scenario_providers,
            project_store,
            operation,
        }
    }

    /// Creates the native executor with upstream provider composition enabled.
    ///
    /// - `runner`: Native root composition owning the provider factory.
    /// - `service`: Shared application boundary for semantic tools.
    /// - `context`: Trusted caller capabilities and scope.
    /// - `session`: Exact session owning this plan.
    /// - `project_store`: Durable project catalog and candidate store.
    /// - `operation`: Shared coordinator operation used for cancellation.
    ///
    /// # Returns
    /// The configured worker executor.
    pub(super) fn with_upstream_runner(
        runner: &'a super::NativeTuiAgentRunner,
        service: &'a ApplicationService<B>,
        context: &'a ApplicationContext,
        session: EntityId,
        project_store: Arc<SqliteStore>,
        operation: Option<scriberig_app::OperationId>,
    ) -> Self {
        Self {
            runner: Some(runner),
            service,
            context,
            session,
            #[cfg(test)]
            scenario_providers: &runner.providers,
            project_store,
            operation,
        }
    }
}

impl<B> PlanWorkerExecutor for NativePlanWorkerExecutor<'_, B>
where
    B: ApplicationBackend + Send + Sync + 'static,
{
    /// Rejects execution that lacks the exact plan worker context.
    ///
    /// - `node`: Claimed plan node.
    /// - `claim`: Exclusive worker claim.
    /// - `execution`: Resolved model assignment.
    /// - `_node`: Unused because native execution requires compiled context.
    /// - `_claim`: Unused because native execution requires compiled context.
    /// - `_execution`: Unused because native execution requires compiled context.
    ///
    /// This executor requires `execute_with_context` so no worker silently runs
    /// without its exact continuation and dependency inputs.
    /// # Returns
    /// An error explaining that the exact context is required.
    /// # Errors
    /// Always returns an error.
    fn execute(
        &self,
        _node: &PlanNode,
        _claim: &WorkerClaim,
        _execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        Err("native plan workers require their compiled exact context".to_owned())
    }

    /// Runs the provider and semantic tools inside the claimed isolated branch.
    ///
    /// - `node`: Exact durable work node selected by the coordinator.
    /// - `claim`: Exclusive branch claim persisted before provider execution.
    /// - `execution`: Resolved provider, model, profile, and thinking settings.
    /// - `worker_context`: Exact continuation, ancestor, dependency, scope, and
    ///   local branch context compiled by the shared runtime.
    ///
    /// A compact durable result with safe model provenance handles.
    ///
    /// Returns provider, tool, cancellation, or worker-context failures.
    /// # Returns
    /// The durable branch result.
    /// # Errors
    /// Returns an error when worker execution fails.
    fn execute_with_context(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
        worker_context: &scriberig_agent_runtime::plan_context::PlanWorkerContext,
    ) -> Result<BranchResult, String> {
        self.execute_with_context_and_requests(node, claim, execution, worker_context)
            .map(|execution| execution.result)
    }

    /// Runs a claimed worker and returns its bounded request projections.
    ///
    /// - `node`: Exact durable work node selected by the coordinator.
    /// - `claim`: Exclusive branch claim persisted before provider execution.
    /// - `execution`: Resolved provider, model, profile, and thinking settings.
    /// - `worker_context`: Exact compiled branch context.
    ///
    /// The branch result and captured provider-neutral request history.
    /// # Returns
    /// The completed worker result with safe captured requests.
    /// # Errors
    /// Returns provider, tool, cancellation, or worker-context failures.
    fn execute_with_context_and_requests(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
        worker_context: &scriberig_agent_runtime::plan_context::PlanWorkerContext,
    ) -> Result<PlanWorkerExecutionResult, String> {
        if node
            .expected_result
            .as_ref()
            .is_some_and(|expected| expected.kind == "project_candidate_merge")
        {
            return integrate_dependency_candidates(&self.project_store, claim, worker_context)
                .map(|result| PlanWorkerExecutionResult {
                    result,
                    model_requests: Vec::new(),
                    tool_evidence: Vec::new(),
                });
        }
        let connection = if execution.uses_runtime_fallback {
            "local".to_owned()
        } else {
            super::provider_config::catalog::connection_identity(
                &execution.provider_id,
                &execution.model_id,
            )
        };
        let cancellation = self
            .operation
            .as_ref()
            .map(|operation| self.service.operations().cancellation_flag(operation))
            .transpose()
            .map_err(|error| error.to_string())?;
        let cancelled =
            cancellation.unwrap_or_else(|| Arc::new(std::sync::atomic::AtomicBool::new(false)));
        if let Some(result) = self.execute_upstream_worker(
            node,
            claim,
            execution,
            worker_context,
            &cancelled,
            &connection,
        )? {
            return Ok(result);
        }
        if self.runner.is_some() {
            return Err("selected plan provider has no upstream runtime".to_owned());
        }
        #[cfg(test)]
        if self.runner.is_none() {
            let provider = self
                .scenario_providers
                .get(&connection)
                .cloned()
                .ok_or_else(|| format!("no deterministic scenario for {connection}"))?;
            let provider = provider.into_provider(execution.model.clone());
            return self.execute_bound_upstream_worker(
                node,
                claim,
                execution,
                worker_context,
                &cancelled,
                provider,
                execution.model.clone(),
                None,
            );
        }
        Err("selected plan provider has no upstream runtime".to_owned())
    }
}
