//! Provides local provider-facing agent control tools over shared application state.

use scriberig_agent_core::{InspectTool, Tool, ToolError, ToolInput, ToolResult};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationService, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

use super::agent_tool_helpers::{
    argument_id, argument_operation, invalid_argument, map_error, require_capability,
};

#[path = "project_bootstrap.rs"]
mod project_bootstrap;
#[cfg(test)]
#[path = "project_bootstrap_tests.rs"]
mod project_bootstrap_tests;
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
    /// # Parameters
    /// - `super`: Module namespace containing the tool constructor.
    /// - `service`: Application service owning the shared boundaries.
    ///
    /// # Returns
    /// A tool set for the local agent branch.
    #[must_use]
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
    /// # Parameters
    /// - `store`: Durable store containing project registrations and files.
    /// - `super`: Module namespace containing the tool constructor.
    ///
    /// # Returns
    /// This tool set with project recovery enabled.
    pub(super) fn with_project_store(mut self, store: std::sync::Arc<SqliteStore>) -> Self {
        self.project = Some(ProjectService::new(std::sync::Arc::clone(&store)));
        self.project_store = Some(store);
        self
    }
}

impl<B: ApplicationBackend> Tool for LocalAgentTool<'_, B> {
    /// Returns the primary tool identifier.
    ///
    /// # Returns
    /// The stable composite agent-tool identifier.
    fn name(&self) -> &'static str {
        "scriberig.agent"
    }

    /// Delegates semantic inspection to the shared inspect adapter.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `input`: Entity inspection input.
    ///
    /// # Returns
    /// Compact inspection result.
    ///
    /// # Errors
    /// Returns the shared inspection failure.
    fn invoke(
        &self,
        context: &ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        self.inspect.invoke(context, input)
    }

    /// Executes one application control tool through shared APIs.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `name`: Registered control-tool identifier.
    /// - `arguments`: Provider arguments.
    ///
    /// # Returns
    /// Bounded control result with stable handles.
    ///
    /// # Errors
    /// Returns authorization, parsing, or application failures.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        require_capability(context, name)?;
        match name {
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
            "project_rescan" => self.project_rescan(arguments),
            "project_repair_file" => self.project_repair_file(arguments),
            "session_branch_tree" => self.session_tree(context, arguments),
            "session_branch_activity" => self.session_activity(context, arguments),
            _ => Err(ToolError::Application {
                category: "unsupported_operation".to_owned(),
                message: format!("unknown agent control tool: {name}"),
            }),
        }
    }
}

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Builds a bounded control result from safe text and handles.
    ///
    /// # Parameters
    /// - `summary`: Safe bounded result description.
    /// - `handles`: Stable handles for follow-up requests.
    ///
    /// # Returns
    /// A provider-neutral control result.
    ///
    /// # Errors
    /// This constructor does not fail.
    fn result(summary: impl Into<String>, handles: Vec<String>) -> ToolResult {
        ToolResult {
            entity: EntityId::from_bytes([0; 16]),
            revision: None,
            snapshot: None,
            object_type: Some("agent_control".to_owned()),
            summary: Some(summary.into()),
            handles,
        }
    }

    /// Returns the static provider-neutral runtime capability vocabulary.
    ///
    /// # Returns
    /// A bounded capability result.
    ///
    /// # Errors
    /// This lookup does not fail.
    fn capabilities() -> ToolResult {
        let names = [
            "runtime_capabilities",
            "operation_status",
            "operation_cancel",
            "event_subscription",
            "agent_start",
            "agent_continue",
            "session_branch_tree",
            "session_branch_activity",
            "agent_event_subscription",
            "agent_streaming_output",
            "project_scan",
            "project_manifest",
            "project_import",
            "project_workspace",
            "project_rescan",
            "project_repair_file",
        ];
        Self::result(
            "runtime capabilities available",
            names.iter().map(|name| (*name).to_owned()).collect(),
        )
    }

    /// Reads one operation lifecycle record.
    ///
    /// # Parameters
    /// - `arguments`: Provider arguments containing an operation identifier.
    ///
    /// # Returns
    /// The current operation status and stable operation handle.
    ///
    /// # Errors
    /// Returns an invalid identifier or application lookup failure.
    fn operation_status(&self, arguments: &serde_json::Value) -> Result<ToolResult, ToolError> {
        let id = argument_operation(arguments)?;
        let record = self
            .service
            .operations()
            .get(&id)
            .map_err(|error| map_error(&error))?;
        Ok(Self::result(
            format!("operation {} is {:?}", record.reference, record.status),
            vec![record.reference.to_string()],
        ))
    }

    /// Cancels one shared operation lifecycle record.
    ///
    /// # Parameters
    /// - `arguments`: Provider arguments containing an operation identifier.
    ///
    /// # Returns
    /// The cancelled operation status and stable operation handle.
    ///
    /// # Errors
    /// Returns an invalid identifier or application lifecycle failure.
    fn operation_cancel(&self, arguments: &serde_json::Value) -> Result<ToolResult, ToolError> {
        let id = argument_operation(arguments)?;
        let record = self
            .service
            .operations()
            .cancel(&id)
            .map_err(|error| map_error(&error))?;
        Ok(Self::result(
            format!("operation {} is cancelled", record.reference),
            vec![record.reference.to_string()],
        ))
    }

    /// Reads retained application events with an optional agent filter.
    ///
    /// # Parameters
    /// - `name`: Event subscription tool identifier selecting the filter.
    ///
    /// # Returns
    /// Bounded event cursors represented as stable handles.
    ///
    /// # Errors
    /// This buffered read does not fail.
    fn events(&self, name: &str) -> ToolResult {
        let events = self.service.operations().events_with_sequences();
        let events = events.into_iter().filter(|(_, event)| match name {
            "agent_streaming_output" => {
                matches!(
                    event,
                    scriberig_app::ApplicationEvent::AgentOutputDelta { .. }
                )
            }
            "agent_event_subscription" => matches!(
                event,
                scriberig_app::ApplicationEvent::OperationStarted { .. }
                    | scriberig_app::ApplicationEvent::OperationProgressed { .. }
                    | scriberig_app::ApplicationEvent::OperationCompleted { .. }
                    | scriberig_app::ApplicationEvent::OperationFailed { .. }
                    | scriberig_app::ApplicationEvent::OperationCancelled { .. }
                    | scriberig_app::ApplicationEvent::AgentOutputProduced { .. }
                    | scriberig_app::ApplicationEvent::AgentOutputDelta { .. }
            ),
            _ => true,
        });
        let handles: Vec<String> = events.map(|(sequence, _)| sequence.to_string()).collect();
        let filter = if name == "agent_streaming_output" {
            "agent output deltas"
        } else if name == "agent_event_subscription" {
            "agent lifecycle events"
        } else {
            "application lifecycle events"
        };
        Self::result(format!("{filter}: {} event(s)", handles.len()), handles)
    }

    /// Accepts a new agent command through the shared session boundary.
    ///
    /// # Parameters
    /// - `context`: Trusted application authorization context.
    /// - `arguments`: Provider arguments containing session and prompt.
    /// - `branch`: Optional existing branch for continuation.
    ///
    /// # Returns
    /// Accepted operation and session handles.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, or application failures.
    fn agent_command(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
        branch: Option<EntityId>,
    ) -> Result<ToolResult, ToolError> {
        let session = argument_id(arguments, "session")?;
        let prompt = arguments
            .get("prompt")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_argument("prompt"))?;
        let operation = self
            .service
            .submit_agent_command(
                context,
                &scriberig_app::AgentCommandRequest {
                    session,
                    branch,
                    prompt: prompt.to_owned(),
                    selection: None,
                },
            )
            .map_err(|error| map_error(&error))?;
        Ok(Self::result(
            format!("agent command accepted as {operation}"),
            vec![operation.to_string(), session.to_string()],
        ))
    }

    /// Reads a compact durable session tree.
    ///
    /// # Parameters
    /// - `context`: Trusted application authorization context.
    /// - `arguments`: Provider arguments containing a session identifier.
    ///
    /// # Returns
    /// Branch handles and a bounded session summary.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, or session lookup failures.
    fn session_tree(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let session = argument_id(arguments, "session")?;
        let value = self
            .service
            .read_session(context, &SessionRequest { session })
            .map_err(|error| map_error(&error))?;
        let handles = value
            .branches
            .iter()
            .map(|branch| branch.id.to_string())
            .collect::<Vec<_>>();
        Ok(Self::result(
            format!("session has {} branch(es)", handles.len()),
            handles,
        ))
    }

    /// Reads one branch activity view using the shared session boundary.
    ///
    /// # Parameters
    /// - `context`: Trusted application authorization context.
    /// - `arguments`: Provider arguments containing session and branch IDs.
    ///
    /// # Returns
    /// The selected branch handle and bounded turn count.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, or session lookup failures.
    fn session_activity(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let session = argument_id(arguments, "session")?;
        let branch = argument_id(arguments, "branch")?;
        let value = self
            .service
            .read_session(context, &SessionRequest { session })
            .map_err(|error| map_error(&error))?;
        let selected = value.branches.iter().find(|item| item.id == branch);
        let turns = selected.map_or(0, |item| item.turns.len());
        Ok(Self::result(
            format!("branch {branch} has {turns} turn(s)"),
            vec![branch.to_string()],
        ))
    }
}
