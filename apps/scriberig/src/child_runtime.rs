//! Composes recursive child-agent execution through public application APIs.

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{ProviderAdapter, ToolDefinition};
use scriberig_agent_runtime::{ChildExecutionScope, ChildExecutor, ChildRequest, ChildResult};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationService, Capability, CapabilityAction,
    CapabilityContext,
};
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;
#[path = "child_runtime/control_parameters.rs"]
mod control_parameters;
use control_parameters::control_parameters;
#[path = "child_runtime/handles.rs"]
mod handles;
pub(super) use handles::compact_result_handles;

/// Builds the trusted provider-visible catalog for the local agent runtime.
///
/// # Returns
/// Deterministically ordered inspect, control, and delegation descriptors.
///
/// # Parameters
/// - `super`: Module namespace containing the catalog composition helper.
#[allow(clippy::too_many_lines)]
pub(super) fn inspect_catalog() -> scriberig_agent_runtime::tools::ToolCatalog {
    use scriberig_agent_runtime::tools::{ToolCatalog, ToolDescriptor, ToolExecutionMode};

    let mut descriptors = vec![
        ToolDescriptor {
            definition: ToolDefinition {
                name: "inspect".to_owned(),
                description: "Inspect one semantic entity at an exact snapshot.".to_owned(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity": {"type": "string"},
                        "snapshot": {"type": "string"},
                        "detail": {"type": "string", "enum": ["compact", "detailed"]}
                    },
                    "required": ["entity"]
                }),
            },
            result_shape: "compact entity handles and resolved revision".to_owned(),
            state_changing: false,
            execution_mode: ToolExecutionMode::Atomic,
            required_capability: Some("inspect.entity".to_owned()),
        },
        ToolDescriptor {
            definition: ToolDefinition {
                name: "delegate".to_owned(),
                description: "Delegate focused reasoning to an isolated child branch.".to_owned(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "purpose": {"type": "string"},
                        "prompt": {"type": "string"},
                        "capabilities": {"type": "array", "items": {"type": "string"}},
                        "expected_result": {"type": "string"}
                    },
                    "required": ["purpose", "prompt", "capabilities", "expected_result"]
                }),
            },
            result_shape: "compact child branch result and handles".to_owned(),
            state_changing: false,
            execution_mode: ToolExecutionMode::ChildBranch,
            required_capability: Some("agent.delegate".to_owned()),
        },
    ];
    for (name, description, required_capability) in [
        (
            "runtime_capabilities",
            "Discover exposed runtime capabilities.",
            "runtime_capabilities",
        ),
        (
            "operation_status",
            "Read one operation lifecycle status.",
            "operation_status",
        ),
        (
            "operation_cancel",
            "Cancel one active operation.",
            "operation_cancel",
        ),
        (
            "event_subscription",
            "Read ordered lifecycle events.",
            "event_subscription",
        ),
        (
            "agent_start",
            "Accept agent work for a session.",
            "agent_start",
        ),
        (
            "agent_continue",
            "Continue work on a session branch.",
            "agent_continue",
        ),
        (
            "session_branch_tree",
            "Read a durable session branch tree.",
            "session_branch_tree",
        ),
        (
            "session_branch_activity",
            "Read ordered branch activity.",
            "session_branch_activity",
        ),
        (
            "agent_event_subscription",
            "Read ordered agent lifecycle events.",
            "agent_event_subscription",
        ),
        (
            "agent_streaming_output",
            "Read bounded agent output events.",
            "agent_streaming_output",
        ),
        (
            "project_scan",
            "Scan the trusted current working directory into a persisted project manifest.",
            "project_scan",
        ),
        (
            "project_manifest",
            "Review a bounded page of an exact persisted project scan manifest.",
            "project_manifest",
        ),
        (
            "project_import",
            "Import the exact reviewed scan manifest and return semantic workspace references.",
            "project_import",
        ),
        (
            "project_workspace",
            "Resolve the current exact semantic workspace for an imported project.",
            "project_workspace",
        ),
        (
            "semantic_find",
            "Find project-owned source and declaration references in one exact snapshot.",
            "semantic_find",
        ),
        (
            "semantic_render",
            "Render one project source through an exact semantic entity reference.",
            "semantic_render",
        ),
        (
            "project_rescan",
            "Rescan a project after a source repair.",
            "project_rescan",
        ),
        (
            "project_repair_file",
            "Repair one authorized project-relative source file.",
            "project_repair_file",
        ),
        (
            "project_propose",
            "Propose an exact-base multi-file project source candidate.",
            "project_propose",
        ),
        (
            "project_candidate",
            "Inspect a durable project-wide candidate and its path effects.",
            "project_candidate",
        ),
        (
            "project_validate_candidate",
            "Validate one exact project candidate in an isolated workspace.",
            "project_validate_candidate",
        ),
    ] {
        descriptors.push(ToolDescriptor {
            definition: ToolDefinition {
                name: name.to_owned(),
                description: description.to_owned(),
                parameters: control_parameters(name),
            },
            result_shape: "bounded provider-neutral control result".to_owned(),
            state_changing: matches!(
                name,
                "operation_cancel"
                    | "agent_start"
                    | "agent_continue"
                    | "project_scan"
                    | "project_import"
                    | "project_rescan"
                    | "project_repair_file"
                    | "project_propose"
                    | "project_validate_candidate"
            ),
            execution_mode: ToolExecutionMode::Atomic,
            required_capability: Some(required_capability.to_owned()),
        });
    }
    ToolCatalog::new(descriptors)
}

/// Executes recursively delegated branches through the public application API.
pub(super) struct LocalChildExecutor<'a, B, P> {
    /// Application service owning session and operation state.
    pub(super) service: &'a ApplicationService<B>,
    /// Cloneable provider adapter used for each child request.
    pub(super) provider: P,
    /// Model identifier shared by child provider requests.
    pub(super) model: String,
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
    /// # Parameters
    /// - `context`: Authorized application context inherited by the child.
    /// - `scope`: Durable session and parent branch identity.
    /// - `request`: Validated child purpose and prompt.
    /// - `cancelled`: Shared cancellation flag for the execution tree.
    ///
    /// # Returns
    /// Compact child summary and durable result handle.
    ///
    /// # Errors
    /// Returns a safe failure when child creation or execution fails.
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
            provider: self.provider.clone(),
            model: self.model.clone(),
            cache: self.cache.clone(),
            project_store: std::sync::Arc::clone(&self.project_store),
        };
        let child_capabilities =
            delegated_capabilities(&context.capabilities, &request.capabilities);
        let child_context = ApplicationContext {
            capabilities: child_capabilities.clone(),
            ..context.clone()
        };
        let mut model = ProviderModel::new(self.provider.clone(), self.model.clone())
            .with_catalog(inspect_catalog())
            .with_capabilities(child_capabilities)
            .with_cache(self.cache.clone(), CachePolicy::ReuseExact)
            .with_cache_namespace(child_scope.parent_branch.to_string());
        if let Some(semantic_scope) = child_scope.semantic_scope.clone() {
            let compiled = scriberig_agent_runtime::context::ContextCompiler::new(
                scriberig_agent_runtime::context::ContextPolicy::default(),
            )
            .compile(
                &scriberig_agent_runtime::context::BranchExecutionContext::new(
                    child_scope.session.to_string(),
                    child_scope.parent_branch.to_string(),
                    &request.purpose,
                    &request.prompt,
                )
                .with_project_scope(semantic_scope),
            );
            model = model.with_compiled_context(compiled);
        }
        let result = scriberig_agent_runtime::AgentRuntime::new(
            model,
            LocalAgentTool::new(self.service)
                .with_project_store(std::sync::Arc::clone(&self.project_store)),
            32,
        )
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
            )
            .map_err(|error| error.to_string())?;
        Ok(ChildResult {
            summary: completed.final_output,
            handles: std::iter::once(completed.branch_result.to_string())
                .chain(result_handles)
                .collect(),
        })
    }
}

/// Restricts child grants to the explicitly delegated capability names.
///
/// # Parameters
/// - `parent`: Capabilities available to the parent branch.
/// - `requested`: Capability identifiers delegated by the parent model.
///
/// # Returns
/// A child capability context that is no broader than the parent request.
fn delegated_capabilities(parent: &CapabilityContext, requested: &[String]) -> CapabilityContext {
    let requested = requested
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let granted = parent
        .granted
        .iter()
        .filter(|capability| requested.contains(capability.as_str()))
        .cloned()
        .collect();
    let grants = parent
        .grants
        .iter()
        .filter_map(|grant| {
            let actions = grant
                .actions
                .iter()
                .filter(|action| match action {
                    CapabilityAction::Inspect => requested.contains("inspect.entity"),
                    CapabilityAction::Query => requested.contains("query"),
                    CapabilityAction::Custom(name) => requested.contains(name.as_str()),
                    _ => false,
                })
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            (!actions.is_empty()).then(|| Capability {
                actions,
                ..grant.clone()
            })
        })
        .collect();
    CapabilityContext { granted, grants }
}

#[cfg(test)]
#[path = "child_runtime_tests.rs"]
mod tests;
