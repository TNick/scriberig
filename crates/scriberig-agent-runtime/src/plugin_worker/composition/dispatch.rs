//! Dispatches one authorized plugin tool call and records its durable evidence.

use std::sync::Arc;

use rpi_agent::{AgentError, AgentToolResult, TextContentOrImage};
use rpi_extensions::isolated::WorkerError;
use scriberig_app::ToolExecutionEvidence;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::plugin_worker::AuthorizedPluginWorker;

use super::bounded::{
    MAX_EVIDENCE_INPUT_BYTES, MAX_PLUGIN_ARGUMENT_BYTES, bounded_failure, bounded_retained,
};

/// Runs one worker tool off the async loop and projects its worker error.
///
/// # Parameters
/// - `worker`: Authorized worker that must advertise and allow this tool.
/// - `name`: Tool name bound to this adapter.
/// - `arguments`: Arguments validated by `rpi-agent` against the schema.
/// - `cancellation`: Branch-scoped tool cancellation token.
///
/// # Returns
/// A text tool result for the provider-visible projection.
///
/// # Errors
/// Returns an upstream abort for a cancellation and a safe tool error for a
/// bound, unauthorized, timed-out, or failed worker call.
pub(super) async fn call_worker(
    worker: Arc<AuthorizedPluginWorker>,
    name: &str,
    arguments: Value,
    cancellation: CancellationToken,
) -> Result<AgentToolResult, AgentError> {
    if cancellation.is_cancelled() {
        return Err(AgentError::Abort);
    }
    if serde_json::to_vec(&arguments).is_ok_and(|bytes| bytes.len() > MAX_PLUGIN_ARGUMENT_BYTES) {
        return Err(AgentError::tool(
            "plugin tool arguments exceed the branch limit",
        ));
    }
    if !worker.allows(name) || !worker.advertises(name) {
        return Err(AgentError::tool(
            "plugin tool is not authorized for this branch",
        ));
    }
    let called = {
        let name = name.to_owned();
        tokio::task::spawn_blocking(move || {
            worker.call_tool_cancellable(&name, arguments, Some(&cancellation))
        })
        .await
    };
    match called {
        Err(_) => Err(AgentError::tool("plugin worker call failed")),
        Ok(Err(WorkerError::Cancelled)) => Err(AgentError::Abort),
        Ok(Err(error)) => Err(AgentError::tool(bounded_failure(&error.to_string()))),
        Ok(Ok(result)) => Ok(AgentToolResult::text(result.to_string())),
    }
}

/// Builds the durable evidence record for one settled worker call.
///
/// # Parameters
/// - `ordinal`: Branch activity position for this settled call.
/// - `name`: Trusted plugin tool identifier.
/// - `arguments`: Validated provider arguments.
/// - `result`: Settled tool result or safe failure.
///
/// # Returns
/// Bounded structured evidence for the branch activity list.
pub(super) fn plugin_evidence(
    ordinal: u64,
    name: &str,
    arguments: &Value,
    result: &Result<AgentToolResult, AgentError>,
) -> ToolExecutionEvidence {
    let failed = result.is_err();
    let mut input = serde_json::to_vec(arguments).unwrap_or_default();
    if input.len() > MAX_EVIDENCE_INPUT_BYTES {
        input = br#"{"truncated":true}"#.to_vec();
    }
    let summary = match result {
        Ok(tool_result) => {
            let mut text = String::new();
            for content in &tool_result.content {
                if let TextContentOrImage::Text(content) = content {
                    text.push_str(&content.text);
                }
            }
            text
        }
        Err(error) => bounded_failure(&error.to_string()),
    };
    let output = serde_json::to_vec(&Value::String(bounded_retained(&summary))).unwrap_or_default();
    ToolExecutionEvidence {
        ordinal,
        name: name.to_owned(),
        input,
        presentation_input: None,
        presentation_label: Some(name.to_owned()),
        category: Some(scriberig_session::ToolActivityKind::OpaqueFallback),
        output,
        failed,
        child_branch: None,
    }
}
