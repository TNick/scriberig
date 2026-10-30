//! Scripts a provider-neutral PHP self-extension scenario for product gates.

use rpi_ai::{AssistantMessage, Context, Model};
use scriberig_agent_runtime::host_configuration::HostConfigurationError;
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

impl PhpSelfHostingProvider {
    /// Produces the next semantic scenario message from the upstream context.
    ///
    /// - `context`: Exact upstream context supplied for the provider call.
    /// - `model`: Selected upstream model that will emit the response.
    ///
    /// # Returns
    /// The next scripted response, or a safe scenario failure.
    ///
    /// # Errors
    /// Returns an error when required semantic scope or scenario state is absent.
    pub(super) fn respond(
        self,
        context: &Context,
        model: &Model,
    ) -> Result<AssistantMessage, String> {
        let prompt = user_prompt(context);
        if prompt.to_lowercase().contains("onboard") {
            if self.cancellation == Some(CancellationPhase::Onboarding) {
                await_test_release().map_err(|error| error.to_string())?;
            }
            return BootstrapProvider::respond(context, model);
        }
        let phase_delay = self.cancellation == Some(CancellationPhase::Discovery)
            && prompt.contains("ARCHITECTURE_DISCOVERY_TASK");
        let implementation_delay = self.cancellation == Some(CancellationPhase::Implementation)
            && prompt.contains("PHP_IMPLEMENTATION_TASK");
        if phase_delay || implementation_delay {
            await_test_release().map_err(|error| error.to_string())?;
        }
        let scope = project_scope(context).map_err(|error| error.to_string())?;
        if !prompt.to_lowercase().contains("php") {
            return Err("PHP self-hosting scenario received an unexpected request".to_owned());
        }
        let is_architecture = prompt.contains("ARCHITECTURE_DISCOVERY_TASK");
        let is_implementation = prompt.contains("PHP_IMPLEMENTATION_TASK");
        if is_architecture {
            return branches::architecture_branch(context, model, &scope)
                .map_err(|error| error.to_string());
        }
        if is_implementation {
            return branches::implementation_branch(context, model, &scope, self.validation)
                .map_err(|error| error.to_string());
        }
        branches::root_branch(
            context,
            model,
            &scope,
            self.validation != ValidationMode::Normal,
        )
        .map_err(|error| error.to_string())
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
/// Holds a cancellation scenario response until the PTY test releases its synchronization gate.
///
/// Unit after the test writes its release marker, or immediately outside gated PTY tests.
///
/// Returns a provider configuration error when the configured release marker is invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn await_test_release() -> Result<(), HostConfigurationError> {
    let Ok(path) = std::env::var("SCRIBE_AGENT_TEST_PROVIDER_GATE") else {
        return Ok(());
    };
    let path = std::path::PathBuf::from(path);
    let waiting = path.with_extension("waiting");
    std::fs::write(&waiting, b"provider reached the cancellation gate").map_err(|error| {
        HostConfigurationError::Scenario(format!("cannot mark scenario gate as waiting: {error}"))
    })?;
    loop {
        match std::fs::metadata(&path) {
            Ok(_) => {
                std::fs::remove_file(&waiting).map_err(|error| {
                    HostConfigurationError::Scenario(format!(
                        "cannot remove provider gate waiting marker: {error}"
                    ))
                })?;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => {
                return Err(HostConfigurationError::Scenario(format!(
                    "cannot inspect test provider release marker: {error}"
                )));
            }
        }
    }
}

/// Extracts the user prompt from a normalized request.
///
/// - `context`: Current provider request.
///
/// - `super`: Parent module visibility marker.
///   First user message or an empty string.
/// # Returns
/// The operation result.
pub(super) fn user_prompt(context: &Context) -> String {
    crate::scenario_helpers::user_prompt(context)
}

/// Returns normalized tool result contents in conversation order.
///
/// - `context`: Current provider request.
///
/// - `super`: Parent module visibility marker.
///   Tool observation content slices.
/// # Returns
/// The operation result.
pub(super) fn tool_messages(context: &Context) -> Vec<String> {
    crate::scenario_helpers::tool_messages(context)
}

/// Reads exact project/snapshot scope from compiled branch context.
///
/// - `context`: Current provider request.
///
/// - `super`: Parent module visibility marker.
///   Exact semantic project and snapshot references.
///
/// Returns a configuration failure when compiled semantic scope is missing.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn project_scope(context: &Context) -> Result<SemanticScope, HostConfigurationError> {
    let system_prompt = context.system_prompt.as_deref().unwrap_or_default();
    let scope = system_prompt
        .split_once("semantic_project_scope:")
        .map(|(_, scope)| scope)
        .ok_or_else(|| {
            HostConfigurationError::Scenario("exact project scope is missing".to_owned())
        })?;
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
pub(super) fn semantic_entity(message: &str) -> Result<String, HostConfigurationError> {
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
/// - `model`: Upstream model producing the tool call.
///
/// - `super`: Parent module visibility marker.
///   Normal tool-call response.
/// # Returns
/// The operation result.
pub(super) fn tool_call(model: &Model, name: &str, arguments: Value) -> AssistantMessage {
    crate::scenario_helpers::tool_call(model, name, arguments)
}

/// Creates a normal child-branch request through the model tool contract.
///
/// - `purpose`: Durable child purpose.
/// - `prompt`: Explicit child instructions.
/// - `capabilities`: Narrow child capabilities.
/// - `expected_result`: Compact `BranchResult` contract.
/// - `model`: Upstream model producing the delegation request.
///
/// - `super`: Parent module visibility marker.
///   Provider response requesting a nested branch.
/// # Returns
/// The operation result.
pub(super) fn delegate(
    model: &Model,
    purpose: &str,
    prompt: &str,
    capabilities: &[&str],
    expected_result: &str,
) -> AssistantMessage {
    tool_call(
        model,
        "delegate",
        json!({"purpose":purpose,"prompt":prompt,"capabilities":capabilities,"expected_result":expected_result}),
    )
}

/// Creates a concise terminal model response.
///
/// - `text`: Durable assistant result text.
/// - `model`: Upstream model producing the final response.
///
/// - `super`: Parent module visibility marker.
///   Normal stop response.
/// # Returns
/// The operation result.
pub(super) fn final_response(model: &Model, text: &str) -> AssistantMessage {
    crate::scenario_helpers::final_response(model, text)
}

/// Creates a normalized malformed-response error.
///
/// - `message`: Safe explanation of missing scripted evidence.
///
/// - `super`: Parent module visibility marker.
///   Provider malformed-response error.
/// # Returns
/// The operation result.
pub(super) fn malformed(message: &str) -> HostConfigurationError {
    HostConfigurationError::Scenario(message.to_owned())
}
