//! Scripts a provider-neutral PHP self-extension scenario for product gates.

use scriberig_agent_runtime::provider::{
    FinishReason, MessageRole, ModelRequest, ModelResponse, ProviderAdapter, ProviderError,
    ProviderMetadata, ToolCall,
};
use serde_json::{Value, json};

use crate::deterministic_provider::{BootstrapProvider, parse_handles};

/// Deterministic model scenario that proposes PHP support through ordinary tools.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PhpSelfHostingProvider {
    /// Whether this deterministic scenario deliberately proposes invalid Rust.
    pub validation_failure: bool,
}

mod branches;
mod candidate;

/// Exact immutable project scope observed by the deterministic model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SemanticScope {
    /// Selected project identity.
    pub project: String,
    /// Exact selected immutable snapshot.
    pub snapshot: String,
}

impl ProviderAdapter for PhpSelfHostingProvider {
    /// Returns the next ordinary tool call for onboarding or PHP implementation.
    ///
    /// # Parameters
    /// - `self`: Configured deterministic product-gate scenario.
    /// - `request`: Normalized model request from the shared runtime.
    ///
    /// # Returns
    /// One normal provider tool call or a concise branch result.
    ///
    /// # Errors
    /// Returns an error when exact project scope or expected tool results are absent.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        let prompt = user_prompt(request);
        if prompt.to_lowercase().contains("onboard") {
            return BootstrapProvider.complete(request);
        }
        let scope = project_scope(request)?;
        if !prompt.to_lowercase().contains("php") {
            return Err(ProviderError::Configuration(
                "PHP self-hosting scenario received an unexpected request".to_owned(),
            ));
        }
        let is_architecture = prompt.contains("ARCHITECTURE_DISCOVERY_TASK");
        let is_implementation = prompt.contains("PHP_IMPLEMENTATION_TASK");
        if is_architecture {
            return branches::architecture_branch(request, &scope);
        }
        if is_implementation {
            return branches::implementation_branch(request, &scope, self.validation_failure);
        }
        branches::root_branch(request, &scope, self.validation_failure)
    }
}

/// Extracts the user prompt from a normalized request.
///
/// # Parameters
/// - `request`: Current provider request.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// First user message or an empty string.
pub(super) fn user_prompt(request: &ModelRequest) -> &str {
    request
        .messages
        .iter()
        .rev()
        .find(|message| message.role == MessageRole::User)
        .map_or("", |message| message.content.as_str())
}

/// Returns normalized tool result contents in conversation order.
///
/// # Parameters
/// - `request`: Current provider request.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// Tool observation content slices.
pub(super) fn tool_messages(request: &ModelRequest) -> Vec<&str> {
    request
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::Tool)
        .map(|message| message.content.as_str())
        .collect()
}

/// Reads exact project/snapshot scope from compiled branch context.
///
/// # Parameters
/// - `request`: Current provider request.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// Exact semantic project and snapshot references.
///
/// # Errors
/// Returns a configuration failure when compiled semantic scope is missing.
pub(super) fn project_scope(request: &ModelRequest) -> Result<SemanticScope, ProviderError> {
    let content = request
        .messages
        .iter()
        .find(|message| message.role == MessageRole::System)
        .map(|message| message.content.as_str())
        .unwrap_or_default();
    let scope = content
        .split_once("semantic_project_scope:")
        .map(|(_, scope)| scope)
        .ok_or_else(|| ProviderError::Configuration("exact project scope is missing".to_owned()))?;
    let value = |key: &str| {
        scope
            .split_whitespace()
            .find_map(|part| part.strip_prefix(&format!("{key}=")))
            .map(str::to_owned)
            .ok_or_else(|| malformed(&format!("{key} scope missing")))
    };
    Ok(SemanticScope {
        project: value("project")?,
        snapshot: value("snapshot")?,
    })
}

/// Finds the semantic source entity handle in a find result.
///
/// # Parameters
/// - `message`: Normalized find tool result.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// Stable source entity identifier.
///
/// # Errors
/// Returns malformed-response when no source entity is present.
pub(super) fn semantic_entity(message: &str) -> Result<String, ProviderError> {
    parse_handles(message)?
        .iter()
        .filter_map(|handle| serde_json::from_str::<Value>(handle).ok())
        .find(|value| value.get("kind").and_then(Value::as_str) == Some("source"))
        .and_then(|value| value.get("entity")?.as_str().map(str::to_owned))
        .ok_or_else(|| malformed("semantic source handle missing"))
}

/// Finds one handle with a stable prefix in a normalized tool result.
///
/// # Parameters
/// - `message`: Provider tool message.
/// - `prefix`: Stable handle prefix.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// Handle suffix when present.
pub(super) fn handle_with_prefix(message: &str, prefix: &str) -> Option<String> {
    parse_handles(message)
        .ok()?
        .into_iter()
        .find_map(|handle| handle.strip_prefix(prefix).map(str::to_owned))
}

/// Creates one provider-neutral named tool call.
///
/// # Parameters
/// - `name`: Registered trusted tool identifier.
/// - `arguments`: Structured tool arguments.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// Normal tool-call response.
pub(super) fn tool_call(name: &str, arguments: Value) -> ModelResponse {
    ModelResponse {
        text: None,
        tool_calls: vec![ToolCall {
            id: format!("self-host-{name}"),
            name: name.to_owned(),
            arguments,
        }],
        finish_reason: FinishReason::ToolCall,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

/// Creates a normal child-branch request through the model tool contract.
///
/// # Parameters
/// - `purpose`: Durable child purpose.
/// - `prompt`: Explicit child instructions.
/// - `capabilities`: Narrow child capabilities.
/// - `expected_result`: Compact `BranchResult` contract.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// Provider response requesting a nested branch.
pub(super) fn delegate(
    purpose: &str,
    prompt: &str,
    capabilities: &[&str],
    expected_result: &str,
) -> ModelResponse {
    tool_call(
        "delegate",
        json!({"purpose":purpose,"prompt":prompt,"capabilities":capabilities,"expected_result":expected_result}),
    )
}

/// Creates a concise terminal model response.
///
/// # Parameters
/// - `text`: Durable assistant result text.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// Normal stop response.
pub(super) fn final_response(text: &str) -> ModelResponse {
    ModelResponse {
        text: Some(text.to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

/// Creates a normalized malformed-response error.
///
/// # Parameters
/// - `message`: Safe explanation of missing scripted evidence.
///
/// - `super`: Parent module visibility marker.
/// # Returns
/// Provider malformed-response error.
pub(super) fn malformed(message: &str) -> ProviderError {
    ProviderError::MalformedResponse(message.to_owned())
}
