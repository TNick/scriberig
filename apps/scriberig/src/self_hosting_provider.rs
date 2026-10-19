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
    /// Validation behavior selected for the first candidate.
    pub validation: ValidationMode,
    /// Optional provider call held for phase cancellation coverage.
    pub cancellation: Option<CancellationPhase>,
}

/// Selects whether the deterministic provider proposes a failed first candidate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum ValidationMode {
    /// Propose and validate one valid candidate.
    #[default]
    Normal,
    /// Propose an invalid candidate and leave it reviewable after failure.
    FailFirst,
    /// Repair the invalid first candidate through a second normal proposal.
    RepairFirst,
}

/// Identifies a delayed provider call used by real TUI cancellation tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CancellationPhase {
    /// Delay initial project onboarding.
    Onboarding,
    /// Delay architecture discovery before its first semantic read.
    Discovery,
    /// Delay implementation before its first semantic reinspection.
    Implementation,
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
    /// - `self`: Configured deterministic product-gate scenario.
    /// - `request`: Normalized model request from the shared runtime.
    ///
    /// One normal provider tool call or a concise branch result.
    ///
    /// Returns an error when exact project scope or expected tool results are absent.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        let prompt = user_prompt(request);
        if prompt.to_lowercase().contains("onboard") {
            if self.cancellation == Some(CancellationPhase::Onboarding) {
                await_test_release()?;
            }
            return BootstrapProvider.complete(request);
        }
        let phase_delay = self.cancellation == Some(CancellationPhase::Discovery)
            && request_contains(request, "ARCHITECTURE_DISCOVERY_TASK");
        let implementation_delay = self.cancellation == Some(CancellationPhase::Implementation)
            && request_contains(request, "PHP_IMPLEMENTATION_TASK");
        if phase_delay || implementation_delay {
            await_test_release()?;
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
            return branches::implementation_branch(request, &scope, self.validation);
        }
        branches::root_branch(request, &scope, self.validation != ValidationMode::Normal)
    }
}

/// Checks whether any normalized request message contains a scenario task anchor.
///
/// - `request`: Normalized provider request.
/// - `anchor`: Stable task marker to find.
///
/// `true` when the marker occurs in any request message.
/// # Returns
/// The operation result.
fn request_contains(request: &ModelRequest, anchor: &str) -> bool {
    request
        .messages
        .iter()
        .any(|message| message.content.contains(anchor))
}

/// Holds a cancellation scenario response until the PTY test releases its synchronization gate.
///
/// Unit after the test writes its release marker, or immediately outside gated PTY tests.
///
/// Returns a provider configuration error when the configured release marker is invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn await_test_release() -> Result<(), ProviderError> {
    let Ok(path) = std::env::var("SCRIBE_AGENT_TEST_PROVIDER_GATE") else {
        return Ok(());
    };
    let path = std::path::PathBuf::from(path);
    let waiting = path.with_extension("waiting");
    std::fs::write(&waiting, b"provider reached the cancellation gate").map_err(|error| {
        ProviderError::Configuration(format!("cannot mark provider gate as waiting: {error}"))
    })?;
    loop {
        match std::fs::metadata(&path) {
            Ok(_) => {
                std::fs::remove_file(&waiting).map_err(|error| {
                    ProviderError::Configuration(format!(
                        "cannot remove provider gate waiting marker: {error}"
                    ))
                })?;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => {
                return Err(ProviderError::Configuration(format!(
                    "cannot inspect test provider release marker: {error}"
                )));
            }
        }
    }
}

/// Extracts the user prompt from a normalized request.
///
/// - `request`: Current provider request.
///
/// - `super`: Parent module visibility marker.
///   First user message or an empty string.
/// # Returns
/// The operation result.
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
/// - `request`: Current provider request.
///
/// - `super`: Parent module visibility marker.
///   Tool observation content slices.
/// # Returns
/// The operation result.
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
/// - `request`: Current provider request.
///
/// - `super`: Parent module visibility marker.
///   Exact semantic project and snapshot references.
///
/// Returns a configuration failure when compiled semantic scope is missing.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `message`: Normalized find tool result.
///
/// - `super`: Parent module visibility marker.
///   Stable source entity identifier.
///
/// Returns malformed-response when no source entity is present.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `message`: Provider tool message.
/// - `prefix`: Stable handle prefix.
///
/// - `super`: Parent module visibility marker.
///   Handle suffix when present.
/// # Returns
/// The operation result.
pub(super) fn handle_with_prefix(message: &str, prefix: &str) -> Option<String> {
    parse_handles(message)
        .ok()?
        .into_iter()
        .find_map(|handle| handle.strip_prefix(prefix).map(str::to_owned))
}

/// Creates one provider-neutral named tool call.
///
/// - `name`: Registered trusted tool identifier.
/// - `arguments`: Structured tool arguments.
///
/// - `super`: Parent module visibility marker.
///   Normal tool-call response.
/// # Returns
/// The operation result.
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
/// - `purpose`: Durable child purpose.
/// - `prompt`: Explicit child instructions.
/// - `capabilities`: Narrow child capabilities.
/// - `expected_result`: Compact `BranchResult` contract.
///
/// - `super`: Parent module visibility marker.
///   Provider response requesting a nested branch.
/// # Returns
/// The operation result.
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
/// - `text`: Durable assistant result text.
///
/// - `super`: Parent module visibility marker.
///   Normal stop response.
/// # Returns
/// The operation result.
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
/// - `message`: Safe explanation of missing scripted evidence.
///
/// - `super`: Parent module visibility marker.
///   Provider malformed-response error.
/// # Returns
/// The operation result.
pub(super) fn malformed(message: &str) -> ProviderError {
    ProviderError::MalformedResponse(message.to_owned())
}
