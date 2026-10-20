//! Defines credential-safe model descriptors exposed to clients.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Identifies an upstream model input modality safe for public display.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelInputModality {
    /// Text input is supported.
    Text,
    /// Image input is supported.
    Image,
}

/// Carries typed pricing metadata from the upstream safe snapshot.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ModelCostSnapshot {
    /// Cost per input token unit.
    pub input: f64,
    /// Cost per output token unit.
    pub output: f64,
    /// Cost per cached input token unit read.
    pub cache_read: f64,
    /// Cost per cached input token unit written.
    pub cache_write: f64,
    /// Optional pricing tiers for larger inputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tiers: Vec<ModelCostTierSnapshot>,
}

/// Describes one upstream pricing tier.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ModelCostTierSnapshot {
    /// Cost per input token unit at this tier.
    pub input: f64,
    /// Cost per output token unit at this tier.
    pub output: f64,
    /// Cost per cached input token unit read at this tier.
    pub cache_read: f64,
    /// Cost per cached input token unit written at this tier.
    pub cache_write: f64,
    /// Minimum input-token count for this tier.
    pub input_tokens_above: i64,
}

/// Carries display-safe metadata from upstream's versioned model snapshot.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ModelMetadataSnapshot {
    /// Upstream snapshot schema identifier.
    pub schema_version: String,
    /// Public provider API identifier.
    pub api_id: String,
    /// Whether upstream marks the model as reasoning-capable.
    pub reasoning: bool,
    /// Input modalities supported by the model.
    pub input: Vec<ModelInputModality>,
    /// Typed pricing metadata from upstream.
    pub cost: ModelCostSnapshot,
    /// Whether pricing was explicitly configured upstream.
    pub cost_configured: bool,
    /// Upstream context-window limit in tokens.
    pub context_window: u64,
    /// Upstream maximum output-token limit.
    pub max_tokens: u64,
}

/// Describes a provider model without resolved credentials.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ModelDescriptor {
    /// Stable Scriberig model id.
    pub id: String,
    /// Safe human-readable model name.
    pub display_name: String,
    /// Upstream provider id scoped to this model.
    #[serde(alias = "provider_profile")]
    pub provider_id: String,
    /// Provider-native model id needed to explain historical invocations.
    pub provider_model_id: String,
    /// Upstream thinking-level identifiers supported by this model, in order.
    pub thinking_levels: Vec<String>,
    /// Safe metadata copied from the upstream versioned snapshot.
    pub metadata_snapshot: ModelMetadataSnapshot,
}

/// Returns safe model rows projected from the upstream catalog.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ModelLibraryResponse {
    /// Safe upstream-derived model display rows.
    pub models: Vec<ModelDescriptor>,
}
