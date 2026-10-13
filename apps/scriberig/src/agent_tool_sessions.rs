//! Provides durable session inspection operations to agent control tools.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_app::{ApplicationBackend, ApplicationContext, SessionBackend, SessionRequest};

use super::LocalAgentTool;
use crate::agent_tool_helpers::{argument_id, map_error};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Reads a compact durable session tree.
    ///
    /// # Parameters
    /// - `super`: Shared local agent tool.
    /// - `context`: Trusted application authorization context.
    /// - `arguments`: Provider arguments containing a session identifier.
    ///
    /// # Returns
    /// Branch handles and a bounded session summary.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, or session lookup failures.
    pub(super) fn session_tree(
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
    /// - `super`: Shared local agent tool.
    /// - `context`: Trusted application authorization context.
    /// - `arguments`: Provider arguments containing session and branch IDs.
    ///
    /// # Returns
    /// The selected branch handle and bounded turn count.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, or session lookup failures.
    pub(super) fn session_activity(
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
