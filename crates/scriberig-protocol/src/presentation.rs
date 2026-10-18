//! Defines shared agent-console presentation preference protocol values.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Controls safe reasoning summaries and runtime status visibility.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningVisibility {
    /// Omit safe summaries and status rows.
    #[default]
    Hidden,
    /// Show safe user-facing summaries and status rows.
    Shown,
}

/// Controls captured tool invocation detail.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolInvocationDetail {
    /// Show a compact command-like line.
    #[default]
    Simple,
    /// Show bounded schema-redacted arguments.
    Detailed,
}

/// Controls captured tool result visibility.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutputVisibility {
    /// Hide result content but retain status.
    Hidden,
    /// Show safe deterministic summaries.
    #[default]
    Summary,
    /// Show bounded preview and expansion controls.
    Full,
}

/// Controls captured model-request visibility.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptVisibility {
    /// Omit captured model-request sections.
    #[default]
    Hidden,
    /// Show bounded captured model-request sections.
    Shown,
}

/// Shared default presentation choices for already captured activity.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentPresentationPreferences {
    /// Safe reasoning-summary visibility.
    pub reasoning: ReasoningVisibility,
    /// Captured tool invocation detail.
    pub tool_invocation: ToolInvocationDetail,
    /// Captured tool output verbosity.
    pub tool_output: ToolOutputVisibility,
    /// Captured model-request visibility.
    pub prompt: PromptVisibility,
}

/// Reads a caller's shared presentation preferences.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentPresentationPreferencesReadRequest {}

/// Replaces shared presentation defaults at an observed revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentPresentationPreferencesSetRequest {
    /// Preference revision observed by the caller.
    pub expected_revision: u64,
    /// Complete independent presentation choices.
    pub preferences: AgentPresentationPreferences,
}

/// Revisioned shared presentation defaults for one authorized caller.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentPresentationPreferencesResponse {
    /// Current preference mutation revision.
    pub revision: u64,
    /// Shared default choices.
    pub preferences: AgentPresentationPreferences,
}
