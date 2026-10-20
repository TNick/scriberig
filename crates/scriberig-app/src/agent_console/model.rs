//! Defines provider-neutral model library descriptors and thinking capabilities.

pub use super::library::ModelLibrary;

#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
/// Identifies a model in a user or project model library.
///
/// # Public members
/// - `new`: Creates a validated stable model identity.
/// - `as_str`: Returns the stable model identity text.
///
/// # Private members
/// - The wrapped string stores the stable identity.
pub struct ModelId(String);

impl ModelId {
    /// Creates a model identifier from a non-empty stable value.
    ///
    /// - `value`: Stable model identity.
    ///
    /// The validated model identifier.
    ///
    /// Returns an error when the identifier is empty or whitespace-only.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("model id must not be empty");
        }
        Ok(Self(value))
    }

    /// Returns the stable model identifier text.
    ///
    /// The identifier value.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
/// Wraps an upstream thinking level for Scriberig assignment storage.
///
/// # Public members
/// - `new`: Parses an upstream thinking-level name.
/// - `as_str`: Returns its canonical upstream name.
///
/// # Private members
/// - The wrapped upstream enum stores the canonical level.
#[serde(transparent)]
pub struct ThinkingLevelId(rpi_ai::ThinkingLevel);

impl ThinkingLevelId {
    /// Parses a canonical upstream thinking-level name.
    ///
    /// - `value`: Upstream thinking-level name.
    ///
    /// The validated thinking identifier.
    ///
    /// Returns an error when the name is not supported upstream.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        rpi_ai::ThinkingLevel::parse_name(&value)
            .map(Self)
            .ok_or("thinking level id is not supported by upstream")
    }

    /// Returns the canonical upstream thinking-level name.
    ///
    /// The identifier value.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &'static str {
        self.0.as_str()
    }
}

/// Mirrors display-safe metadata emitted by the upstream model snapshot.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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

/// Identifies an input modality in the upstream safe model snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelInputModality {
    /// Text input is supported.
    Text,
    /// Image input is supported.
    Image,
}

/// Contains the typed pricing rates from an upstream model snapshot.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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
    pub tiers: Vec<ModelCostTierSnapshot>,
}

/// Describes one upstream pricing tier.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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

/// Describes a provider model without exposing provider credentials.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelDescriptor {
    /// Stable Scriberig model identity.
    pub id: ModelId,
    /// Safe human-readable model name.
    pub display_name: String,
    /// Upstream provider identity scoped to this model.
    pub provider_id: String,
    /// Provider-native model identifier.
    pub provider_model_id: String,
    /// Upstream thinking levels supported by this model, in upstream order.
    pub thinking_levels: Vec<rpi_ai::ThinkingLevel>,
    /// Safe display metadata returned by the upstream versioned snapshot.
    pub metadata_snapshot: ModelMetadataSnapshot,
}

#[cfg(test)]
mod tests {
    use super::{
        ModelCostSnapshot, ModelDescriptor, ModelId, ModelInputModality, ModelMetadataSnapshot,
    };

    /// Creates a descriptor with the supplied model-specific levels.
    ///
    /// - `id`: Stable model identity.
    /// - `levels`: Ordered model-specific thinking identifiers.
    ///
    /// A model row with the supplied upstream thinking levels.
    /// # Returns
    /// The operation result.
    fn model(id: &str, levels: &[&str]) -> ModelDescriptor {
        let thinking_levels = levels
            .iter()
            .map(|id| rpi_ai::ThinkingLevel::parse_name(id).expect("valid thinking id"))
            .collect::<Vec<_>>();
        ModelDescriptor {
            id: ModelId::new(id).expect("valid model id"),
            display_name: id.to_owned(),
            provider_id: "provider-safe-id".to_owned(),
            provider_model_id: id.to_owned(),
            thinking_levels,
            metadata_snapshot: ModelMetadataSnapshot {
                schema_version: "rpi-safe-model-snapshot-v1".to_owned(),
                api_id: "openai-completions".to_owned(),
                reasoning: false,
                input: vec![ModelInputModality::Text],
                cost: ModelCostSnapshot {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    tiers: Vec::new(),
                },
                cost_configured: false,
                context_window: 0,
                max_tokens: 0,
            },
        }
    }

    /// Verifies thinking levels are model-specific and need not share names.
    #[test]
    fn models_advertise_independent_thinking_levels() {
        let first = model("model-a", &["minimal", "low"]);
        let second = model("model-b", &["off", "medium", "max"]);

        assert_eq!(first.thinking_levels.len(), 2);
        assert_eq!(second.thinking_levels.len(), 3);
        assert_eq!(second.thinking_levels[2].as_str(), "max");
    }
}
