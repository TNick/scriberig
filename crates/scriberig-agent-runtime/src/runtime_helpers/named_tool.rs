//! Executes named tools and captures safe invocation presentation metadata.

use std::sync::atomic::AtomicBool;

use scriberig_agent_core::Tool;
use scriberig_app::{ApplicationContext, OperationId};

use crate::orchestration::BranchRuntimeState;
use crate::runtime::evidence::tool_evidence_record;
use crate::{AgentError, ModelProvider};

use super::super::AgentRuntime;

impl<P, T> AgentRuntime<P, T>
where
    P: ModelProvider,
    T: Tool,
{
    /// Executes a named tool and pairs its result with safe presentation data.
    ///
    /// - `context`: Trusted application context.
    /// - `name`: Registered tool identifier.
    /// - `arguments`: Exact arguments passed to the application tool.
    /// - `presentation_arguments`: Schema-redacted arguments for durable display.
    /// - `presentation_label`: Optional trusted compact display label.
    /// - `category`: Trusted semantic category from the registered descriptor.
    /// - `cancelled`: Shared cancellation flag.
    /// - `operation`: Optional lifecycle operation.
    /// - `state`: Mutable branch counters.
    ///
    /// The tool result and its durable invocation evidence.
    ///
    /// Returns a tool, cancellation, or budget failure.
    #[allow(clippy::too_many_arguments)]
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(in crate::runtime) fn run_named_tool_with_evidence(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        presentation_arguments: &serde_json::Value,
        presentation_label: Option<&str>,
        category: scriberig_session::ToolActivityKind,
        cancelled: &AtomicBool,
        operation: Option<&OperationId>,
        state: &mut BranchRuntimeState,
    ) -> Result<
        (
            scriberig_agent_core::ToolResult,
            scriberig_app::ToolExecutionEvidence,
        ),
        AgentError,
    > {
        let result = self.run_named_tool(context, name, arguments, cancelled, operation, state)?;
        let evidence = tool_evidence_record(
            name,
            arguments,
            presentation_arguments,
            presentation_label,
            Some(category),
            &result,
        );
        Ok((result, evidence))
    }
}
