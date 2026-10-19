//! Converts normalized provider tool calls into runtime decisions.

use crate::provider::{ToolCall, ToolDefinition};
use crate::{ChildRequest, ModelDecision};

/// Converts a named tool call using its trusted schema for display redaction.
///
/// - `tools`: Trusted provider-visible tool definitions.
/// - `call`: Validated provider tool call.
/// - `presentation_label`: Optional trusted compact label for display.
/// - `category`: Trusted semantic category of the registered tool.
///
/// A runtime tool decision or a delegated child request.
///
/// Returns an error when a child call omits required fields.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn named_tool_decision(
    tools: &[ToolDefinition],
    call: ToolCall,
    presentation_label: Option<String>,
    category: scriberig_session::ToolActivityKind,
) -> Result<ModelDecision, String> {
    if call.name == "delegate" {
        return delegate_decision(&call);
    }
    let schema = tools
        .iter()
        .find(|tool| tool.name == call.name)
        .map(|tool| &tool.parameters);
    let presentation_arguments =
        super::presentation::redact_sensitive_arguments(&call.arguments, schema);
    Ok(ModelDecision::NamedTool {
        name: call.name,
        arguments: call.arguments,
        presentation_arguments,
        presentation_label,
        category,
    })
}

/// Converts a delegation call into its compact child execution contract.
///
/// - `call`: Validated delegate tool call.
///
/// A child request without retaining its provider tool envelope.
///
/// Returns an error when required delegation fields are absent.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn delegate_decision(call: &ToolCall) -> Result<ModelDecision, String> {
    let purpose = required_string(call, "purpose")?;
    let prompt = required_string(call, "prompt")?;
    let capabilities = call
        .arguments
        .get("capabilities")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect();
    let expected_result = call
        .arguments
        .get("expected_result")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("compact result")
        .to_owned();
    Ok(ModelDecision::Child(ChildRequest {
        purpose,
        prompt,
        capabilities,
        expected_result,
    }))
}

/// Reads one required string field from a provider tool call.
///
/// - `call`: Provider tool call containing structured arguments.
/// - `name`: Required argument name.
///
/// The copied argument string.
///
/// Returns an error when the argument is absent or not a string.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn required_string(call: &ToolCall, name: &str) -> Result<String, String> {
    call.arguments
        .get(name)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("delegate tool requires a {name}"))
}
