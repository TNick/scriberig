//! Encodes completed agent tool calls for durable session records.

use crate::{ChildRequest, ChildResult};

/// Encodes one successful application tool execution for durable session storage.
///
/// # Parameters
/// - `name`: Stable provider-facing tool identifier.
/// - `input`: Exact JSON arguments submitted to the tool.
/// - `result`: Structured application tool result.
/// - `super`: Runtime module visibility context.
///
/// # Returns
/// Durable invocation/result evidence.
pub(super) fn tool_evidence_record(
    name: &str,
    input: &serde_json::Value,
    result: &scriberig_agent_core::ToolResult,
) -> scriberig_app::ToolExecutionEvidence {
    scriberig_app::ToolExecutionEvidence {
        name: name.to_owned(),
        input: encode(input),
        output: encode(&serde_json::json!({
            "entity": result.entity.to_string(),
            "revision": result.revision.map(|revision| revision.to_string()),
            "snapshot": result.snapshot.map(|snapshot| snapshot.to_string()),
            "object_type": result.object_type,
            "summary": result.summary,
            "handles": result.handles,
        })),
        failed: false,
    }
}

/// Encodes one completed child delegation as bounded durable tool evidence.
///
/// # Parameters
/// - `request`: Explicit child delegation contract.
/// - `result`: Compact child result returned to this branch.
/// - `super`: Runtime module visibility context.
///
/// # Returns
/// Durable invocation/result evidence for the delegate tool.
pub(super) fn child_evidence_record(
    request: &ChildRequest,
    result: &ChildResult,
) -> scriberig_app::ToolExecutionEvidence {
    scriberig_app::ToolExecutionEvidence {
        name: "delegate".to_owned(),
        input: encode(&serde_json::json!({
            "purpose": request.purpose,
            "prompt": request.prompt,
            "capabilities": request.capabilities,
            "expected_result": request.expected_result,
        })),
        output: encode(&serde_json::json!({
            "summary": result.summary,
            "handles": result.handles,
        })),
        failed: false,
    }
}

/// Serializes a JSON value for durable provider-neutral evidence.
///
/// # Parameters
/// - `value`: JSON value to encode.
///
/// # Returns
/// UTF-8 JSON bytes; JSON values are always serializable.
fn encode(value: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("JSON values serialize without failure")
}
