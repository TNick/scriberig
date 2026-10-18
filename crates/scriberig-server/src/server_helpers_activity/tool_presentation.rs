//! Projects bounded safe tool arguments and result details.

/// Builds the public identifier for a projected activity item.
///
/// # Parameters
/// - `kind`: Public category of the activity.
/// - `has_no_parent`: Whether the source activity has no causal parent.
/// - `id`: Durable source entity identity.
///
/// # Returns
/// Stable identifier with a role suffix for human and standalone output rows.
pub(super) fn projected_activity_id(
    kind: scriberig_protocol::SessionActivityKind,
    has_no_parent: bool,
    id: scriberig_core::EntityId,
) -> String {
    match (kind, has_no_parent) {
        (scriberig_protocol::SessionActivityKind::HumanInput, _) => format!("{id}:human"),
        (scriberig_protocol::SessionActivityKind::AssistantOutput, true) => {
            format!("{id}:assistant")
        }
        _ => id.to_string(),
    }
}

/// Projects bounded safe detail for one tool activity.
///
/// # Parameters
/// - `activity`: Durable activity reference being projected.
/// - `branch`: Branch containing the corresponding tool record.
///
/// # Returns
/// Safe invocation arguments or result preview, with the other value absent.
pub(super) fn activity_tool_details(
    activity: &scriberig_session::ExecutionActivity,
    branch: &scriberig_session::Branch,
) -> (Option<serde_json::Value>, Option<serde_json::Value>) {
    match activity {
        scriberig_session::ExecutionActivity::ToolInvocation(id) => (
            branch
                .tool_invocations
                .iter()
                .find(|invocation| invocation.id == *id)
                .and_then(|invocation| invocation.presentation_input.as_deref())
                .and_then(bounded_tool_arguments),
            None,
        ),
        scriberig_session::ExecutionActivity::ToolResult(id) => (
            None,
            branch
                .tool_results
                .iter()
                .find(|result| result.id == *id)
                .and_then(|result| bounded_tool_result_preview(&result.output)),
        ),
        _ => (None, None),
    }
}

/// Bounds display text by Unicode scalar values.
///
/// # Parameters
/// - `value`: Externally meaningful text.
///
/// # Returns
/// At most 512 characters with an ellipsis when truncated.
pub(super) fn bounded_summary(value: &str) -> String {
    let mut chars = value.chars();
    let mut summary = chars.by_ref().take(512).collect::<String>();
    if chars.next().is_some() {
        summary.push('…');
    }
    summary
}

/// Decodes and bounds persisted schema-redacted tool argument detail.
///
/// # Parameters
/// - `bytes`: Safe structured argument bytes stored with the invocation.
///
/// # Returns
/// The parsed argument value when its JSON form is at most 4096 characters.
pub(super) fn bounded_tool_arguments(bytes: &[u8]) -> Option<serde_json::Value> {
    let value = serde_json::from_slice::<serde_json::Value>(bytes).ok()?;
    (value.to_string().chars().count() <= 4096).then_some(value)
}

/// Projects only safe result metadata for bounded full-mode display.
///
/// # Parameters
/// - `bytes`: Structured result bytes retained by the trusted tool boundary.
///
/// # Returns
/// A whitelisted object bounded to 4096 serialized characters.
pub(super) fn bounded_tool_result_preview(bytes: &[u8]) -> Option<serde_json::Value> {
    let value = serde_json::from_slice::<serde_json::Value>(bytes).ok()?;
    let object = value.as_object()?;
    let mut preview = serde_json::Map::new();
    for key in ["entity", "revision", "snapshot", "object_type", "summary"] {
        if let Some(serde_json::Value::String(text)) = object.get(key) {
            let bounded = text.chars().take(512).collect::<String>();
            preview.insert(key.to_owned(), serde_json::Value::String(bounded));
        }
    }
    if let Some(serde_json::Value::Array(handles)) = object.get("handles") {
        let handles = handles
            .iter()
            .filter_map(serde_json::Value::as_str)
            .take(20)
            .map(|handle| handle.chars().take(128).collect::<String>())
            .collect::<Vec<_>>();
        preview.insert("handles".to_owned(), serde_json::json!(handles));
    }
    let preview = serde_json::Value::Object(preview);
    (preview.to_string().chars().count() <= 4096).then_some(preview)
}

/// Reads the tool boundary's safe summary, falling back to a generic status.
///
/// # Parameters
/// - `result`: Durable result bytes returned by the trusted tool boundary.
/// - `failed`: Whether the tool execution failed.
///
/// # Returns
/// A bounded result summary without exposing arbitrary output fields.
pub(super) fn tool_result_summary(result: &scriberig_session::ToolResult, failed: bool) -> String {
    let fallback = if failed {
        "Tool failed"
    } else {
        "Tool completed"
    };
    serde_json::from_slice::<serde_json::Value>(&result.output)
        .ok()
        .and_then(|value| {
            value
                .get("summary")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .filter(|summary| !summary.trim().is_empty())
        .map_or_else(|| fallback.to_owned(), |summary| bounded_summary(&summary))
}
