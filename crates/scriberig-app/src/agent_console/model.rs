//! Defines `ScribeRig` model identity and upstream catalog projections.

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

/// Associates `ScribeRig`'s stable identity with an upstream safe model snapshot.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelDescriptor {
    /// Stable `ScribeRig` model identity.
    pub id: ModelId,
    /// Safe display metadata returned by the upstream versioned snapshot.
    pub metadata_snapshot: rpi_model_runtime::SafeModelSnapshot,
}

#[cfg(test)]
mod tests {
    use super::{ModelDescriptor, ModelId};

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
            metadata_snapshot: rpi_model_runtime::SafeModelSnapshot {
                schema_version: "rpi-safe-model-snapshot-v1".to_owned(),
                provider_id: "provider-safe-id".to_owned(),
                model_id: id.to_owned(),
                display_name: id.to_owned(),
                api_id: "openai-completions".to_owned(),
                reasoning: false,
                input: vec![rpi_ai::InputModality::Text],
                cost: rpi_ai::ModelCost::default(),
                cost_configured: false,
                context_window: 0,
                max_tokens: 0,
                thinking_levels,
                compatibility: None,
                excluded_execution_fields: vec![
                    rpi_model_runtime::ModelExecutionExclusion::Endpoint,
                ],
            },
        }
    }
    /// Verifies thinking levels are model-specific and need not share names.
    #[test]
    fn models_advertise_independent_thinking_levels() {
        let first = model("model-a", &["minimal", "low"]);
        let second = model("model-b", &["off", "medium", "max"]);

        assert_eq!(first.metadata_snapshot.thinking_levels.len(), 2);
        assert_eq!(second.metadata_snapshot.thinking_levels.len(), 3);
        assert_eq!(second.metadata_snapshot.thinking_levels[2].as_str(), "max");
    }
}
