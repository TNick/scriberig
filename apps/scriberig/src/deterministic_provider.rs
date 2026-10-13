//! Provides a deterministic model adapter for real application workflow tests.

use scriberig_agent_runtime::provider::{
    FinishReason, MessageRole, ModelRequest, ModelResponse, ProviderAdapter, ProviderError,
    ProviderMetadata, ToolCall,
};

/// Selects real onboarding tools through the normal agent model loop.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct BootstrapProvider;

impl ProviderAdapter for BootstrapProvider {
    /// Returns the next deterministic onboarding tool call or final response.
    ///
    /// # Parameters
    /// - `self`: Deterministic provider scenario.
    /// - `request`: Normalized model request from the shared runtime.
    ///
    /// # Returns
    /// A normal provider response containing one tool call or final text.
    ///
    /// # Errors
    /// Returns a configuration error when a different scenario is requested.
    ///
    /// The ordered transitions are kept explicit to make tool-call evidence auditable.
    #[allow(clippy::too_many_lines)]
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        let prompt = request
            .messages
            .iter()
            .find(|message| message.role == MessageRole::User)
            .map(|message| message.content.as_str())
            .unwrap_or_default();
        if !prompt.to_lowercase().contains("onboard") {
            return semantic_discovery_response(request);
        }
        let tool_messages = request
            .messages
            .iter()
            .filter(|message| message.role == MessageRole::Tool)
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        let response = if tool_messages
            .iter()
            .any(|message| message.contains("active semantic workspace"))
        {
            if tool_messages.len() >= 5 {
                return Ok(ModelResponse {
                    text: Some(
                        "The current project is onboarded and its semantic root was inspected."
                            .to_owned(),
                    ),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    usage: None,
                    metadata: ProviderMetadata::default(),
                });
            }
            let workspace = tool_messages
                .iter()
                .find(|message| message.contains("active semantic workspace"))
                .expect("matched workspace response");
            let handles = parse_handles(workspace)?;
            let snapshot = handles.get(1).ok_or_else(|| {
                ProviderError::MalformedResponse("workspace snapshot is missing".to_owned())
            })?;
            let root = handles.get(2).ok_or_else(|| {
                ProviderError::MalformedResponse("workspace root is missing".to_owned())
            })?;
            tool_call(
                "inspect",
                serde_json::json!({
                    "entity": root,
                    "snapshot": snapshot,
                    "detail": "compact"
                }),
            )
        } else if tool_messages.is_empty() {
            tool_call(
                "project_scan",
                serde_json::json!({
                    "path_ref": "current_working_directory",
                    "git_tracked": true
                }),
            )
        } else if tool_messages
            .iter()
            .any(|message| message.contains("project import succeeded"))
        {
            let scan = tool_messages
                .iter()
                .find(|message| message.contains("scan persisted for"))
                .ok_or_else(|| {
                    ProviderError::MalformedResponse(
                        "project import has no scan reference".to_owned(),
                    )
                })?;
            let project = parse_handles(scan)?.into_iter().next().ok_or_else(|| {
                ProviderError::MalformedResponse("project identity is missing".to_owned())
            })?;
            tool_call("project_workspace", serde_json::json!({"project": project}))
        } else if tool_messages
            .iter()
            .find(|message| message.contains("summary=Some(\"manifest "))
            .is_some()
        {
            let scan = tool_messages
                .iter()
                .find(|message| message.contains("scan persisted for"))
                .ok_or_else(|| {
                    ProviderError::MalformedResponse("scan response is missing".to_owned())
                })?;
            let handles = parse_handles(scan)?;
            let project = handles.first().ok_or_else(|| {
                ProviderError::MalformedResponse("project identity is missing".to_owned())
            })?;
            let manifest_id = handles.get(1).ok_or_else(|| {
                ProviderError::MalformedResponse("manifest identity is missing".to_owned())
            })?;
            tool_call(
                "project_import",
                serde_json::json!({"project": project, "manifest_id": manifest_id}),
            )
        } else if let Some(scan) = tool_messages
            .iter()
            .find(|message| message.contains("scan persisted for"))
        {
            let handles = parse_handles(scan)?;
            if handles.len() < 2 {
                return Err(ProviderError::MalformedResponse(
                    "project scan response omitted project or manifest references".to_owned(),
                ));
            }
            tool_call(
                "project_manifest",
                serde_json::json!({
                    "project": handles[0],
                    "manifest_id": handles[1],
                    "offset": 0,
                    "limit": 100
                }),
            )
        } else {
            return Err(ProviderError::MalformedResponse(
                "onboarding tool sequence did not produce a reviewable manifest".to_owned(),
            ));
        };
        Ok(response)
    }
}

/// Drives exact-snapshot semantic discovery and a read-only child branch.
///
/// # Parameters
/// - `request`: Normalized parent or child model request.
///
/// # Returns
/// A semantic tool call, delegated request, or concise completion.
///
/// # Errors
/// Returns malformed-response when the exact project scope or semantic handles are absent.
fn semantic_discovery_response(request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
    let scope = project_scope(request)?;
    let tools = request
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::Tool)
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>();
    let child = request.messages.iter().any(|message| {
        message.role == MessageRole::User
            && message.content.contains("Find the existing code-domain")
    });
    if child {
        if tools
            .iter()
            .any(|message| message.contains("source_digest:"))
        {
            return Ok(final_response(
                "Use the shared code domain and Tree-sitter adapter pattern; integrate classification, project dispatch, Cargo metadata, registries, and conformance fixtures. Preserve exact bytes, deterministic syntax, diagnostics, and inert parsing.",
            ));
        }
        if let Some(found) = tools
            .iter()
            .find(|message| message.contains("semantic find"))
        {
            let entity = semantic_entity(found)?;
            return Ok(tool_call(
                "semantic_render",
                serde_json::json!({"project": scope.0, "snapshot": scope.1, "entity": entity}),
            ));
        }
        return Ok(tool_call(
            "semantic_find",
            serde_json::json!({
                "project": scope.0,
                "snapshot": scope.1,
                "query": "lib.rs",
                "limit": 10
            }),
        ));
    }
    if tools
        .iter()
        .any(|message| message.contains("child_summary="))
    {
        return Ok(final_response(
            "I found the repository guidance and delegated a bounded architecture map at the same project snapshot.",
        ));
    }
    if let Some(found) = tools
        .iter()
        .find(|message| message.contains("semantic find"))
    {
        if tools
            .iter()
            .any(|message| message.contains("source_digest:"))
        {
            return Ok(tool_call(
                "delegate",
                serde_json::json!({
                    "purpose": "determine minimal extension points for PHP source ingestion",
                    "prompt": "Find the existing code-domain and Tree-sitter language adapter patterns, then summarize the scanner, importer, Cargo, registry, and conformance integration points.",
                    "capabilities": ["inspect.entity", "semantic_find", "semantic_render"],
                    "expected_result": "compact architecture map with exact semantic references"
                }),
            ));
        }
        let entity = semantic_entity(found)?;
        return Ok(tool_call(
            "semantic_render",
            serde_json::json!({"project": scope.0, "snapshot": scope.1, "entity": entity}),
        ));
    }
    Ok(tool_call(
        "semantic_find",
        serde_json::json!({
            "project": scope.0,
            "snapshot": scope.1,
            "query": "AGENTS.md",
            "limit": 10
        }),
    ))
}

/// Reads exact project and snapshot handles from compiled semantic context.
///
/// # Parameters
/// - `request`: Normalized provider request containing compiled branch context.
///
/// # Returns
/// Exact project and snapshot string identities.
///
/// # Errors
/// Returns malformed-response when project scope is missing.
fn project_scope(request: &ModelRequest) -> Result<(String, String), ProviderError> {
    let scope = request
        .messages
        .iter()
        .find(|message| {
            message.role == MessageRole::System
                && message.content.contains("semantic_project_scope:")
        })
        .and_then(|message| message.content.split_once("semantic_project_scope:"))
        .map(|(_, scope)| scope)
        .ok_or_else(|| {
            ProviderError::Configuration(
                "project scope is missing from compiled context".to_owned(),
            )
        })?;
    let value = |key: &str| {
        scope
            .split_whitespace()
            .find_map(|part| part.strip_prefix(&format!("{key}=")))
            .map(str::to_owned)
            .ok_or_else(|| ProviderError::MalformedResponse(format!("{key} scope is missing")))
    };
    Ok((value("project")?, value("snapshot")?))
}

/// Extracts a semantic entity from a find result's stable JSON handle.
///
/// # Parameters
/// - `message`: Normalized semantic find result.
///
/// # Returns
/// Exact entity identity returned by semantic discovery.
///
/// # Errors
/// Returns malformed-response when no semantic entity handle exists.
fn semantic_entity(message: &str) -> Result<String, ProviderError> {
    let handles = parse_handles(message)?;
    handles
        .iter()
        .filter_map(|handle| serde_json::from_str::<serde_json::Value>(handle).ok())
        .find(|value| value.get("kind").and_then(serde_json::Value::as_str) == Some("source"))
        .and_then(|value| value.get("entity")?.as_str().map(str::to_owned))
        .ok_or_else(|| ProviderError::MalformedResponse("semantic entity is missing".to_owned()))
}

/// Builds a normal deterministic completion without tool calls.
///
/// # Parameters
/// - `text`: Concise assistant response.
///
/// # Returns
/// A normalized stop response.
fn final_response(text: &str) -> ModelResponse {
    ModelResponse {
        text: Some(text.to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

/// Builds a normal normalized response with one named application tool call.
///
/// # Parameters
/// - `name`: Registered tool selected by the deterministic model.
/// - `arguments`: Structured input passed through normal catalog validation.
///
/// # Returns
/// Provider response requesting the named tool.
fn tool_call(name: &str, arguments: serde_json::Value) -> ModelResponse {
    ModelResponse {
        text: None,
        tool_calls: vec![ToolCall {
            id: format!("bootstrap-{name}"),
            name: name.to_owned(),
            arguments,
        }],
        finish_reason: FinishReason::ToolCall,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

/// Extracts stable handles from the shared provider-neutral tool observation.
///
/// # Parameters
/// - `message`: Normalized tool-result message.
///
/// # Returns
/// Tool result handles in their emitted order.
///
/// # Errors
/// Returns malformed-response when a tool result has no handle list.
fn parse_handles(message: &str) -> Result<Vec<String>, ProviderError> {
    let values = message
        .split_once("handles=[")
        .and_then(|(_, value)| value.split_once(']'))
        .map(|(values, _)| values)
        .ok_or_else(|| {
            ProviderError::MalformedResponse("tool result omitted handles".to_owned())
        })?;
    serde_json::from_str(&format!("[{values}]")).map_err(|error| {
        ProviderError::MalformedResponse(format!("invalid tool result handles: {error}"))
    })
}
