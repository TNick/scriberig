//! Defines credential-safe model descriptors exposed to clients.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Describes a model-specific thinking capability.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ThinkingLevelDescriptor {
    /// Stable thinking capability id.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Optional safe explanation.
    pub description: Option<String>,
    /// Stable ordering within the model.
    pub order: u32,
}

/// Describes safe model features available to clients.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelCapabilities {
    /// Whether streamed output is available.
    pub streaming: bool,
    /// Whether structured tool calls are available.
    pub tool_calls: bool,
    /// Whether structured output is available.
    pub structured_output: bool,
}

/// Describes a provider model without resolved credentials.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelDescriptor {
    /// Stable Scriberig model id.
    pub id: String,
    /// Safe human-readable model name.
    pub display_name: String,
    /// Safe provider profile id.
    pub provider_profile: String,
    /// Provider-native model id needed to explain historical invocations.
    pub provider_model_id: String,
    /// Safe model capabilities.
    pub capabilities: ModelCapabilities,
    /// Ordered thinking levels advertised by this model.
    pub thinking_levels: Vec<ThinkingLevelDescriptor>,
    /// Advertised default thinking capability.
    pub default_thinking_level: Option<String>,
}

/// Identifies a credential reference without its resolved value.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CredentialRef {
    /// Safe secret-store reference id.
    pub id: String,
}

/// Describes one curated model entry available for selection.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelProfile {
    /// Stable curated profile id.
    pub id: String,
    /// Safe display name.
    pub display_name: String,
    /// Stable model id selected by this profile.
    pub model: String,
    /// Safe connection reference.
    pub connection: String,
    /// Optional credential reference only.
    pub credential: Option<CredentialRef>,
    /// Optional supported default thinking level.
    pub default_thinking_level: Option<String>,
    /// Whether clients may select this profile.
    pub enabled: bool,
}

/// Returns the safe model library and selectable profiles.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelLibraryResponse {
    /// Safe model descriptors.
    pub models: Vec<ModelDescriptor>,
    /// Curated model profiles.
    pub profiles: Vec<ModelProfile>,
}
