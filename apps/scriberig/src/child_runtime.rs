//! Composes recursive child-agent execution through public application APIs.
use scriberig_agent_runtime::provider::ProviderAdapter;

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::{ChildExecutionScope, ChildExecutor, ChildRequest, ChildResult};
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;
#[path = "child_runtime/capability_filter.rs"]
mod capability_filter;
#[path = "child_runtime/construction.rs"]
mod construction;
#[path = "child_runtime/control_parameters.rs"]
mod control_parameters;
use capability_filter::delegated_capabilities;
#[path = "child_runtime/handles.rs"]
mod handles;
pub(super) use handles::compact_result_handles;

#[path = "child_runtime/catalog.rs"]
mod catalog;
pub(super) use catalog::inspect_catalog;

/// Executes recursively delegated branches through the public application API.
pub(super) struct LocalChildExecutor<'a, B, P> {
    /// Application service owning session and operation state.
    pub(super) service: &'a ApplicationService<B>,
    /// Root operation that owns transient child activity.
    pub(super) operation: Option<scriberig_app::OperationId>,
    /// Cloneable provider adapter used for each child request.
    pub(super) provider: P,
    /// Model identifier shared by child provider requests.
    pub(super) model: String,
    /// Thinking capability snapshotted for this execution tree.
    pub(super) thinking_level: Option<String>,
    /// Shared exact-response cache scoped by branch namespace.
    pub(super) cache: ModelCache,
    /// Shared project store used by nested recovery tools.
    pub(super) project_store: std::sync::Arc<SqliteStore>,
}

impl<B, P> ChildExecutor for LocalChildExecutor<'_, B, P>
where
    B: ApplicationBackend,
    P: ProviderAdapter + Clone,
{
    /// Creates, executes, and completes one durable child branch.
    ///
    /// - `context`: Authorized application context inherited by the child.
    /// - `scope`: Durable session and parent branch identity.
    /// - `request`: Validated child purpose and prompt.
    /// - `cancelled`: Shared cancellation flag for the execution tree.
    ///
    /// Compact child summary and durable result handle.
    ///
    /// Returns a safe failure when child creation or execution fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute_child(
        &mut self,
        context: &ApplicationContext,
        scope: Option<ChildExecutionScope>,
        request: ChildRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ChildResult, String> {
        let scope = scope.ok_or_else(|| "durable child scope is missing".to_owned())?;
        let prompt = request.prompt.clone();
        let handle = self
            .service
            .start_local_child_agent(
                scope.session,
                scope.parent_branch,
                "delegated agent branch",
                format!(
                    "{}; expected result: {}",
                    request.purpose, request.expected_result
                ),
                prompt.clone(),
            )
            .map_err(|error| error.to_string())?;
        let child_scope = ChildExecutionScope {
            session: handle.session,
            parent_branch: handle.child_branch,
            depth: scope.depth + 1,
            active_branches: std::sync::Arc::clone(&scope.active_branches),
            semantic_scope: scope.semantic_scope.clone(),
        };
        let mut nested = Self {
            service: self.service,
            operation: self.operation.clone(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            thinking_level: self.thinking_level.clone(),
            cache: self.cache.clone(),
            project_store: std::sync::Arc::clone(&self.project_store),
        };
        let child_capabilities =
            delegated_capabilities(&context.capabilities, &request.capabilities);
        let child_context = ApplicationContext {
            capabilities: child_capabilities.clone(),
            ..context.clone()
        };
        let model = ProviderModel::new(self.provider.clone(), self.model.clone())
            .with_thinking_level(self.thinking_level.clone())
            .with_catalog(inspect_catalog())
            .with_capabilities(child_capabilities)
            .with_cache(self.cache.clone(), CachePolicy::ReuseExact)
            .with_cache_namespace(child_scope.parent_branch.to_string());
        let model = with_child_context(model, &child_scope, &request);
        let result = scriberig_agent_runtime::AgentRuntime::new(
            model,
            LocalAgentTool::new(self.service)
                .with_project_store(std::sync::Arc::clone(&self.project_store)),
            64,
        )
        .with_branch_id(handle.child_branch.to_string());
        let result = if let Some(operation) = &self.operation {
            result.with_observed_operation(self.service.operations().clone(), operation.clone())
        } else {
            result
        }
        .run_with_child_executor_at(
            &child_scope,
            &child_context,
            prompt,
            cancelled,
            &mut nested,
        );
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let message = error.to_string();
                let _ = self.service.fail_local_child_agent(&handle, &message);
                return Err(message);
            }
        };
        let result_handles = compact_result_handles(&result);
        let completed = self
            .service
            .complete_local_child_agent_with_evidence(
                &handle,
                result.output,
                result_handles.clone(),
                result.tool_evidence,
                result.model_requests,
            )
            .map_err(|error| error.to_string())?;
        Ok(ChildResult {
            summary: completed.final_output,
            handles: std::iter::once(completed.branch_result.to_string())
                .chain(result_handles)
                .collect(),
            branch: Some(handle.child_branch),
        })
    }
}

/// Adds a bounded project context to one delegated child model request.
///
/// - `model`: Configured provider model for this child branch.
/// - `scope`: Child session, branch, and optional semantic scope.
/// - `request`: Validated child purpose and prompt.
///
/// The model with compiled context when a semantic scope is present.
fn with_child_context<P>(
    model: ProviderModel<P>,
    scope: &ChildExecutionScope,
    request: &ChildRequest,
) -> ProviderModel<P> {
    let Some(semantic_scope) = scope.semantic_scope.clone() else {
        return model;
    };
    let compiled = scriberig_agent_runtime::context::ContextCompiler::new(
        scriberig_agent_runtime::context::ContextPolicy::default(),
    )
    .compile(
        &scriberig_agent_runtime::context::BranchExecutionContext::new(
            scope.session.to_string(),
            scope.parent_branch.to_string(),
            &request.purpose,
            &request.prompt,
        )
        .with_project_scope(semantic_scope),
    );
    model.with_compiled_context(compiled)
}

#[cfg(test)]
#[path = "child_runtime_tests.rs"]
mod tests;
