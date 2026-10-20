//! Executes claimed plan branches through the native semantic agent runtime.

use std::collections::BTreeMap;
use std::sync::Arc;

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::scheduler::{PlanWorkerExecutionResult, PlanWorkerExecutor};
use scriberig_agent_runtime::{AgentRuntime, ChildExecutionScope};
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_core::EntityId;
use scriberig_session::{BranchResult, BranchStatus, PlanNode, WorkerClaim};
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;
use crate::child_runtime::{LocalChildExecutor, compact_result_handles, inspect_catalog};
use crate::native_provider::NativeProvider;

/// Executes one claimed plan node with its resolved assignment and context.
pub(super) struct NativePlanWorkerExecutor<'a, B> {
    /// Shared application boundary for semantic tools and durable children.
    service: &'a ApplicationService<B>,
    /// Trusted context inherited from the lifecycle request.
    context: &'a ApplicationContext,
    /// Exact session owning this plan.
    session: EntityId,
    /// Configured adapters keyed by safe model-library connection IDs.
    providers: &'a BTreeMap<String, NativeProvider>,
    /// Durable project catalog used by semantic candidate operations.
    project_store: Arc<SqliteStore>,
    /// Shared cancellable coordinator operation.
    operation: Option<scriberig_app::OperationId>,
}

impl<'a, B> NativePlanWorkerExecutor<'a, B> {
    /// Creates the native executor for one coordinator-owned plan run.
    ///
    /// - `service`: Shared application boundary for semantic tools.
    /// - `context`: Trusted caller capabilities and scope.
    /// - `session`: Exact session owning this plan.
    /// - `providers`: Configured adapters keyed by safe connection ID.
    /// - `project_store`: Durable project catalog and candidate store.
    /// - `operation`: Shared coordinator operation used for cancellation.
    ///
    /// An executor whose worker contexts remain branch-local.
    /// # Returns
    /// The configured worker executor.
    pub(super) fn new(
        service: &'a ApplicationService<B>,
        context: &'a ApplicationContext,
        session: EntityId,
        providers: &'a BTreeMap<String, NativeProvider>,
        project_store: Arc<SqliteStore>,
        operation: Option<scriberig_app::OperationId>,
    ) -> Self {
        Self {
            service,
            context,
            session,
            providers,
            project_store,
            operation,
        }
    }
}

impl<B> PlanWorkerExecutor for NativePlanWorkerExecutor<'_, B>
where
    B: ApplicationBackend + Sync,
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
        let provider = self.providers.get(&connection).ok_or_else(|| {
            format!("native plan runtime has no adapter for connection {connection}")
        })?;
        let cancellation = self
            .operation
            .as_ref()
            .map(|operation| self.service.operations().cancellation_flag(operation))
            .transpose()
            .map_err(|error| error.to_string())?;
        let cancelled =
            cancellation.unwrap_or_else(|| Arc::new(std::sync::atomic::AtomicBool::new(false)));
        let scope = ChildExecutionScope {
            session: self.session,
            parent_branch: claim.branch,
            depth: 0,
            active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            semantic_scope: worker_context.inputs.project_scope.clone(),
        };
        let cache = ModelCache::new();
        let mut children = LocalChildExecutor {
            service: self.service,
            operation: self.operation.clone(),
            provider: provider.clone(),
            model: execution.model_id.clone(),
            provider_id: Some(execution.provider_id.clone()),
            provider_profile: None,
            thinking_level: execution.thinking_level.clone(),
            cache: cache.clone(),
            project_store: Arc::clone(&self.project_store),
        };
        let model = ProviderModel::new(provider.clone(), execution.model_id.clone())
            .with_provider_id(execution.provider_id.clone())
            .with_thinking_level(execution.thinking_level.clone())
            .with_catalog(inspect_catalog())
            .with_capabilities(self.context.capabilities.clone())
            .with_cache(cache, CachePolicy::ReuseExact)
            .with_cache_namespace(claim.branch.to_string())
            .with_compiled_context(worker_context.compiled.clone());
        let mut runtime = AgentRuntime::new(
            model,
            LocalAgentTool::new(self.service).with_project_store(Arc::clone(&self.project_store)),
            32,
        )
        .with_branch_id(claim.branch.to_string());
        if let Some(operation) = &self.operation {
            runtime = runtime
                .with_observed_operation(self.service.operations().clone(), operation.clone());
        }
        let result = match runtime.run_with_child_executor_at(
            &scope,
            self.context,
            &node.purpose,
            &cancelled,
            &mut children,
        ) {
            Ok(result) => result,
            Err(error) => {
                let model_requests = runtime.take_captured_model_requests();
                if model_requests.is_empty() {
                    return Err(error.to_string());
                }
                return Ok(PlanWorkerExecutionResult {
                    result: failed_plan_worker_result(claim.branch),
                    model_requests,
                });
            }
        };
        Ok(completed_worker_result(claim.branch, execution, result))
    }
}

/// Builds the durable result for a completed model-backed plan worker.
///
/// - `branch`: Exact worker branch that produced the result.
/// - `execution`: Resolved provider, model, and thinking assignment.
/// - `result`: Runtime output and captured model request projections.
///
/// A completed branch result with safe model provenance handles.
/// # Returns
/// The normalized worker result.
/// # Errors
/// This helper is infallible; `PlanWorkerExecutionResult` is a value type.
fn completed_worker_result(
    branch: EntityId,
    execution: &scriberig_app::ResolvedPlanExecution,
    result: scriberig_agent_runtime::AgentResult,
) -> PlanWorkerExecutionResult {
    let mut result_handles = compact_result_handles(&result);
    result_handles.push(format!("provider:{}", execution.provider_id));
    result_handles.push(format!("model:{}", execution.model_id));
    if let Some(thinking) = &execution.thinking_level {
        result_handles.push(format!("thinking:{thinking}"));
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
    }
}

/// Creates the bounded failed result used when captured requests precede error.
///
/// - `branch`: Exact claimed branch that executed the failed request.
///
/// An abandoned worker result that cannot satisfy plan dependencies.
/// # Returns
/// A failure result with no provider error details.
/// # Errors
/// This helper records failure as data and does not return an error.
fn failed_plan_worker_result(branch: EntityId) -> BranchResult {
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
/// A completed branch result containing the newly merged candidate handle.
///
/// Returns a structured candidate error when dependency inputs are invalid or
/// candidate effects conflict.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when merge inputs are invalid or conflict.
fn integrate_dependency_candidates(
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
