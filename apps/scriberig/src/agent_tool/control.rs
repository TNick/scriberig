//! Implements common agent control operations over the shared application service.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_app::{ApplicationBackend, ApplicationContext, SessionBackend};
use scriberig_core::EntityId;

use super::super::agent_tool_helpers::{
    argument_id, argument_operation, invalid_argument, map_error,
};
use super::LocalAgentTool;

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Builds a bounded control result from safe text and handles.
    ///
    /// - `summary`: Safe bounded result description.
    /// - `handles`: Stable handles for follow-up requests.
    ///
    /// A provider-neutral control result.
    ///
    /// This constructor does not fail.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn result(summary: impl Into<String>, handles: Vec<String>) -> ToolResult {
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
    /// A bounded capability result.
    ///
    /// This lookup does not fail.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn capabilities() -> ToolResult {
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
            "project_propose",
            "project_candidate",
            "project_validate_candidate",
            "project_resolve_dependencies",
        ];
        Self::result(
            "runtime capabilities available",
            names.iter().map(|name| (*name).to_owned()).collect(),
        )
    }

    /// Reads one operation lifecycle record.
    ///
    /// - `arguments`: Provider arguments containing an operation identifier.
    ///
    /// The current operation status and stable operation handle.
    ///
    /// Returns an invalid identifier or application lookup failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn operation_status(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
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
    /// - `arguments`: Provider arguments containing an operation identifier.
    ///
    /// The cancelled operation status and stable operation handle.
    ///
    /// Returns an invalid identifier or application lifecycle failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn operation_cancel(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
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
    /// - `name`: Event subscription tool identifier selecting the filter.
    ///
    /// Bounded event cursors represented as stable handles.
    ///
    /// This buffered read does not fail.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn events(&self, name: &str) -> ToolResult {
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
    /// - `context`: Trusted application authorization context.
    /// - `arguments`: Provider arguments containing session and prompt.
    /// - `branch`: Optional existing branch for continuation.
    ///
    /// Accepted operation and session handles.
    ///
    /// Returns invalid arguments, authorization, or application failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn agent_command(
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
                    anchor: None,
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
}
