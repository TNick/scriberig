//! Encodes completed agent tool calls for durable session records.

use crate::{ChildRequest, ChildResult};

/// Encodes an inspection tool result with safe structured arguments.
///
/// - `input`: Exact semantic inspect request.
/// - `result`: Structured result returned by the trusted inspect tool.
///
/// Durable invocation/result evidence with identical safe input fields.
/// # Returns
/// The operation result.
pub(super) fn inspect_evidence_record(
    input: &scriberig_agent_core::ToolInput,
    result: &scriberig_agent_core::ToolResult,
) -> scriberig_app::ToolExecutionEvidence {
    let arguments = serde_json::json!({
        "entity": input.entity.to_string(),
        "snapshot": input.snapshot.map(|snapshot| snapshot.to_string()),
        "detail": format!("{:?}", input.detail),
    });
    tool_evidence_record(
        "inspect",
        &arguments,
        &arguments,
        None,
        Some(scriberig_session::ToolActivityKind::SemanticRead),
        result,
    )
}

/// Encodes one successful application tool execution for durable session storage.
///
/// - `name`: Stable provider-facing tool identifier.
/// - `input`: Exact JSON arguments submitted to the tool.
/// - `presentation_input`: Schema-redacted JSON arguments safe to display.
/// - `presentation_label`: Optional trusted compact invocation label.
/// - `category`: Trusted semantic category for the durable session record.
/// - `result`: Structured application tool result.
/// - `super`: Runtime module visibility context.
///
/// Durable invocation/result evidence.
/// # Returns
/// The operation result.
pub(super) fn tool_evidence_record(
    name: &str,
    input: &serde_json::Value,
    presentation_input: &serde_json::Value,
    presentation_label: Option<&str>,
    category: Option<scriberig_session::ToolActivityKind>,
    result: &scriberig_agent_core::ToolResult,
) -> scriberig_app::ToolExecutionEvidence {
    scriberig_app::ToolExecutionEvidence {
        ordinal: 0,
        name: name.to_owned(),
        input: encode(input),
        presentation_input: Some(encode(presentation_input)),
        presentation_label: presentation_label.map(str::to_owned),
        category,
        output: encode(&serde_json::json!({
            "entity": result.entity.to_string(),
            "revision": result.revision.map(|revision| revision.to_string()),
            "snapshot": result.snapshot.map(|snapshot| snapshot.to_string()),
            "object_type": result.object_type,
            "summary": result.summary,
            "handles": result.handles,
        })),
        failed: false,
        child_branch: None,
    }
}

/// Encodes one completed child delegation as bounded durable tool evidence.
///
/// - `request`: Explicit child delegation contract.
/// - `result`: Compact child result returned to this branch.
/// - `super`: Runtime module visibility context.
///
/// Durable invocation/result evidence for the delegate tool.
/// # Returns
/// The operation result.
pub(super) fn child_evidence_record(
    request: &ChildRequest,
    result: &ChildResult,
) -> scriberig_app::ToolExecutionEvidence {
    scriberig_app::ToolExecutionEvidence {
        ordinal: 0,
        name: "delegate".to_owned(),
        input: encode(&serde_json::json!({
            "purpose": request.purpose,
            "prompt": request.prompt,
            "capabilities": request.capabilities,
            "expected_result": request.expected_result,
        })),
        presentation_input: None,
        presentation_label: None,
        output: encode(&serde_json::json!({
            "summary": result.summary,
            "handles": result.handles,
        })),
        failed: false,
        child_branch: result.branch,
        category: Some(scriberig_session::ToolActivityKind::BranchProducing),
    }
}

/// Serializes a JSON value for durable provider-neutral evidence.
///
/// - `value`: JSON value to encode.
///
/// UTF-8 JSON bytes; JSON values are always serializable.
/// # Returns
/// The operation result.
fn encode(value: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("JSON values serialize without failure")
}
