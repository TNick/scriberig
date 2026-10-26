//! Provides local provider-facing agent control tools over shared application state.

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_app::agent_tools::InspectTool;
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

use super::agent_tool_helpers::{argument_id, invalid_argument, require_capability};

#[path = "agent_tool/control.rs"]
mod control;
#[path = "agent_tool/domain_operation.rs"]
mod domain_operation;
#[path = "agent_tool/project_candidate_merge.rs"]
mod project_candidate_merge;
#[path = "agent_tool_sessions.rs"]
mod session_operations;

#[path = "project_bootstrap.rs"]
mod project_bootstrap;
#[cfg(test)]
#[path = "project_bootstrap_tests.rs"]
mod project_bootstrap_tests;
#[path = "project_candidates.rs"]
mod project_candidates;
#[path = "project_recovery.rs"]
mod project_recovery;
#[path = "project_semantic.rs"]
mod project_semantic;

/// Provides inspect and application-control tools to one local agent branch.
pub(super) struct LocalAgentTool<'a, B> {
    /// Shared application service owning semantic and lifecycle state.
    pub(super) service: &'a ApplicationService<B>,
    /// Provider-neutral entity inspection adapter.
    pub(super) inspect: InspectTool<&'a ApplicationService<B>>,
    /// Optional project onboarding service exposed to the local agent.
    pub(super) project: Option<ProjectService>,
    /// Optional durable store used for authorized workspace repair.
    pub(super) project_store: Option<std::sync::Arc<SqliteStore>>,
}

impl<'a, B> LocalAgentTool<'a, B> {
    /// Creates a local agent tool set over one application service.
    ///
    /// - `super`: Module namespace containing the tool constructor.
    /// - `service`: Application service owning the shared boundaries.
    ///
    /// A tool set for the local agent branch.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub(super) fn new(service: &'a ApplicationService<B>) -> Self {
        Self {
            service,
            inspect: InspectTool::new(service),
            project: None,
            project_store: None,
        }
    }

    /// Enables project recovery operations over one shared durable store.
    ///
    /// - `store`: Durable store containing project registrations and files.
    /// - `super`: Module namespace containing the tool constructor.
    ///
    /// This tool set with project recovery enabled.
    /// # Returns
    /// The operation result.
    pub(super) fn with_project_store(mut self, store: std::sync::Arc<SqliteStore>) -> Self {
        self.project = Some(ProjectService::new(std::sync::Arc::clone(&store)));
        self.project_store = Some(store);
        self
    }
}

impl<B: ApplicationBackend> Tool<ApplicationContext> for LocalAgentTool<'_, B> {
    /// Returns the primary tool identifier.
    ///
    /// The stable composite agent-tool identifier.
    /// # Returns
    /// The operation result.
    fn name(&self) -> &'static str {
        "scriberig.agent"
    }

    /// Delegates semantic inspection to the shared inspect adapter.
    ///
    /// - `context`: Trusted application context.
    /// - `input`: Entity inspection input.
    ///
    /// Compact inspection result.
    ///
    /// Returns the shared inspection failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn invoke(
        &self,
        context: &ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        self.inspect.invoke(context, input)
    }

    /// Executes one application control tool through shared APIs.
    ///
    /// - `context`: Trusted application context.
    /// - `name`: Registered control-tool identifier.
    /// - `arguments`: Provider arguments.
    /// - `cancelled`: Shared cancellation flag for the active branch.
    ///
    /// Bounded control result with stable handles.
    ///
    /// Returns authorization, parsing, or application failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        self.invoke_named_cancellable(
            context,
            name,
            arguments,
            &std::sync::atomic::AtomicBool::new(false),
        )
    }

    /// Executes one application control tool with cancellation support.
    ///
    /// - `context`: Trusted application context.
    /// - `name`: Registered control-tool identifier.
    /// - `arguments`: Provider arguments.
    /// - `cancelled`: Shared cancellation flag for the active branch.
    ///
    /// Bounded control result with stable handles.
    ///
    /// Returns authorization, parsing, or application failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn invoke_named_cancellable(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        if name != "inspect" {
            require_capability(context, name)?;
        }
        match name {
            "inspect" => {
                let entity = arguments
                    .get("entity")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| invalid_argument("entity"))?
                    .parse::<EntityId>()
                    .map_err(|_| invalid_argument("entity"))?;
                let snapshot = arguments
                    .get("snapshot")
                    .and_then(serde_json::Value::as_str)
                    .map(str::parse::<scriberig_core::SnapshotId>)
                    .transpose()
                    .map_err(|_| invalid_argument("snapshot"))?;
                let detail = match arguments.get("detail").and_then(serde_json::Value::as_str) {
                    Some("detailed") => scriberig_agent_core::ToolDetail::Detailed,
                    _ => scriberig_agent_core::ToolDetail::Compact,
                };
                self.inspect.invoke_cancellable(
                    context,
                    scriberig_agent_core::ToolInput {
                        entity,
                        snapshot,
                        detail,
                    },
                    cancelled,
                )
            }
            "runtime_capabilities" => Ok(Self::capabilities()),
            "operation_status" => self.operation_status(arguments),
            "operation_cancel" => self.operation_cancel(arguments),
            "event_subscription" | "agent_event_subscription" | "agent_streaming_output" => {
                Ok(self.events(name))
            }
            "agent_start" => self.agent_command(context, arguments, None),
            "agent_continue" => {
                let branch = argument_id(arguments, "branch")?;
                self.agent_command(context, arguments, Some(branch))
            }
            "project_scan" => self.project_scan(context, arguments),
            "project_manifest" => self.project_manifest(arguments),
            "project_import" => self.project_import(arguments),
            "project_workspace" => self.project_workspace(arguments),
            "semantic_find" => self.semantic_find(context, arguments),
            "semantic_render" => self.semantic_render(context, arguments),
            "domain_operation" => self.domain_operation(context, arguments),
            "project_rescan" => self.project_rescan(arguments),
            "project_repair_file" => self.project_repair_file(arguments),
            "project_propose" => self.project_propose(context, arguments),
            "project_merge_candidates" => self.project_merge_candidates(arguments),
            "project_candidate" => self.project_candidate(arguments),
            "project_validate_candidate" => self.project_validate_candidate(arguments, cancelled),
            "project_resolve_dependencies" => {
                self.project_resolve_dependencies(arguments, cancelled)
            }
            "session_branch_tree" => self.session_tree(context, arguments),
            "session_branch_activity" => self.session_activity(context, arguments),
            _ => Err(ToolError::Application {
                category: "unsupported_operation".to_owned(),
                message: format!("unknown agent control tool: {name}"),
            }),
        }
    }
}
