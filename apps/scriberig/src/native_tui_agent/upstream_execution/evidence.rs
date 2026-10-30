//! Bounds and records safe `ScribeRig` tool evidence.

use scriberig_agent_runtime::tools::ToolDescriptor;
use scriberig_app::ToolExecutionEvidence;
use scriberig_core::EntityId;

use super::{MAX_TOOL_ARGUMENT_BYTES, MAX_TOOL_OUTPUT_TEXT_BYTES};

/// Creates a `ScribeRig` tool-evidence record from one settled tool call.
///
/// # Parameters
/// - `ordinal`: Monotonic tool execution index within the branch.
/// - `descriptor`: Trusted descriptor for the tool.
/// - `arguments`: Validated provider arguments.
/// - `result`: Safe text projection returned by the semantic operation.
/// - `failed`: Whether the operation returned a failure.
///
/// # Returns
/// Bounded structured durable evidence with trusted category and label.
pub(super) fn tool_evidence(
    ordinal: u64,
    descriptor: &ToolDescriptor,
    arguments: &serde_json::Value,
    result: Option<&(String, Option<EntityId>)>,
    failed: bool,
) -> ToolExecutionEvidence {
    let output = result
        .and_then(|(value, _)| serde_json::from_str::<serde_json::Value>(value).ok())
        .unwrap_or_else(|| serde_json::json!({ "summary": "semantic operation failed" }));
    let mut input = serde_json::to_vec(arguments).unwrap_or_default();
    if input.len() > MAX_TOOL_ARGUMENT_BYTES {
        input = br#"{"truncated":true}"#.to_vec();
    }
    let mut output = serde_json::to_vec(&output).unwrap_or_default();
    if output.len() > MAX_TOOL_OUTPUT_TEXT_BYTES {
        output = serde_json::to_vec(&serde_json::json!({
            "summary": "semantic result exceeded the evidence limit",
            "truncated": true,
        }))
        .unwrap_or_default();
    }
    ToolExecutionEvidence {
        ordinal,
        name: descriptor.definition.name.clone(),
        input,
        presentation_input: None,
        presentation_label: descriptor.invocation_label.clone(),
        category: Some(descriptor.category),
        output,
        failed,
        child_branch: result.and_then(|(_, branch)| *branch),
    }
}

/// Bounds one trusted tool summary by UTF-8 byte length.
///
/// # Parameters
/// - `text`: Safe semantic summary.
/// - `limit`: Maximum number of UTF-8 bytes to retain.
///
/// # Returns
/// A UTF-8 safe prefix at or below the byte limit.
pub(super) fn bounded_text(text: &str, limit: usize) -> String {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}
