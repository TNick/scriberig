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

/// Describes one user-selectable thinking mode supported by a model.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ThinkingLevelDescriptor {
    /// Stable id interpreted by the selected model adapter.
    pub id: ThinkingLevelId,
    /// Human-readable label shown in model controls.
    pub label: String,
    /// Optional explanation of this model-specific level.
    pub description: Option<String>,
    /// Stable presentation order within the model's advertised levels.
    pub order: u32,
}

/// Declares safe behavior available for a model.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelCapabilities {
    /// Whether the model supports streamed output.
    pub streaming: bool,
    /// Whether the model supports structured tool calls.
    pub tool_calls: bool,
    /// Whether the model supports structured output.
    pub structured_output: bool,
}

/// Describes a provider model without exposing provider credentials.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelDescriptor {
    /// Stable Scriberig model identity.
    pub id: ModelId,
    /// Safe human-readable model name.
    pub display_name: String,
    /// Safe provider profile identity.
    pub provider_profile: String,
    /// Provider-native model identifier.
    pub provider_model_id: String,
    /// Safe capabilities advertised by this model.
    pub capabilities: ModelCapabilities,
    /// Ordered model-specific thinking levels.
    pub thinking_levels: Vec<ThinkingLevelDescriptor>,
    /// Model default when it advertises a thinking level.
    pub default_thinking_level: Option<ThinkingLevelId>,
}

/// References a configured credential without containing its resolved value.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CredentialRef {
    /// Safe secret-store reference identifier.
    pub id: String,
}

/// Curates one enabled or disabled entry in a model library.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelProfile {
    /// Stable profile identity.
    pub id: String,
    /// Safe user-facing profile name.
    pub display_name: String,
    /// Model selected by this profile.
    pub model: ModelId,
    /// Provider connection reference, without resolved transport credentials.
    pub connection: String,
    /// Optional credential reference, never a resolved secret value.
    pub credential: Option<CredentialRef>,
    /// Optional profile default supported by the selected model.
    pub default_thinking_level: Option<ThinkingLevelId>,
    /// Whether the profile may be selected for new invocations.
    pub enabled: bool,
}

impl ModelProfile {
    /// Validates the profile's model identity against its descriptor.
    ///
    /// - `model`: Descriptor selected by this profile.
    ///
    /// Returns an error when the profile references another model. Thinking
    /// support and fallback are handled by the upstream model runtime.
    ///
    /// Unit when the profile matches the descriptor.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn validate_for_model(&self, model: &ModelDescriptor) -> Result<(), &'static str> {
        if self.model != model.id {
            return Err("profile model must match the supplied descriptor");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CredentialRef, ModelCapabilities, ModelDescriptor, ModelId, ModelLibrary, ModelProfile,
        ThinkingLevelDescriptor, ThinkingLevelId,
    };

    /// Creates a descriptor with the supplied model-specific levels.
    ///
    /// - `id`: Stable model identity.
    /// - `levels`: Ordered model-specific thinking identifiers.
    ///
    /// A descriptor whose first thinking level is the default.
    /// # Returns
    /// The operation result.
    fn model(id: &str, levels: &[&str]) -> ModelDescriptor {
        let thinking_levels = levels
            .iter()
            .enumerate()
            .map(|(order, id)| ThinkingLevelDescriptor {
                id: ThinkingLevelId::new(*id).expect("valid thinking id"),
                label: (*id).to_owned(),
                description: None,
                order: u32::try_from(order).unwrap_or(u32::MAX),
            })
            .collect::<Vec<_>>();
        ModelDescriptor {
            id: ModelId::new(id).expect("valid model id"),
            display_name: id.to_owned(),
            provider_profile: "provider-safe-id".to_owned(),
            provider_model_id: id.to_owned(),
            capabilities: ModelCapabilities::default(),
            default_thinking_level: thinking_levels.first().map(|level| level.id.clone()),
            thinking_levels,
        }
    }

    /// Verifies thinking levels are model-specific and need not share names.
    #[test]
    fn models_advertise_independent_thinking_levels() {
        let first = model("model-a", &["minimal", "low"]);
        let second = model("model-b", &["off", "medium", "max"]);

        assert_eq!(first.thinking_levels.len(), 2);
        assert_eq!(second.thinking_levels.len(), 3);
        assert_eq!(second.thinking_levels[2].id.as_str(), "max");
    }

    /// Verifies a profile serializes references without resolved credential data.
    #[test]
    fn profile_contains_only_a_credential_reference() {
        let profile = ModelProfile {
            id: "work-profile".to_owned(),
            display_name: "Work model".to_owned(),
            model: ModelId::new("model-a").expect("valid model id"),
            connection: "work-endpoint".to_owned(),
            credential: Some(CredentialRef {
                id: "secret-store:work-provider".to_owned(),
            }),
            default_thinking_level: None,
            enabled: true,
        };
        let serialized = serde_json::to_string(&profile).expect("serialize profile");

        assert!(serialized.contains("secret-store:work-provider"));
        assert!(!serialized.contains("resolved-secret-marker"));
        assert!(!format!("{profile:?}").contains("resolved-secret-marker"));
    }

    /// Verifies profile defaults are not validated against local support lists.
    #[test]
    fn profile_accepts_upstream_level_outside_presentation_list() {
        let descriptor = model("model-a", &["low"]);
        let profile = ModelProfile {
            id: "work-profile".to_owned(),
            display_name: "Work model".to_owned(),
            model: descriptor.id.clone(),
            connection: "work-endpoint".to_owned(),
            credential: None,
            default_thinking_level: Some(ThinkingLevelId::new("max").expect("valid thinking id")),
            enabled: true,
        };

        assert!(profile.validate_for_model(&descriptor).is_ok());
    }

    /// Verifies library entries reference known models and unique identities.
    #[test]
    fn model_library_validates_profile_references() {
        let descriptor = model("model-a", &["low"]);
        let profile = ModelProfile {
            id: "profile-a".to_owned(),
            display_name: "Profile A".to_owned(),
            model: descriptor.id.clone(),
            connection: "connection-a".to_owned(),
            credential: None,
            default_thinking_level: None,
            enabled: true,
        };
        let library = ModelLibrary::new(vec![descriptor.clone()], vec![profile.clone()])
            .expect("valid library");

        assert_eq!(library.model(&descriptor.id), Some(&descriptor));
        assert_eq!(library.profile("profile-a"), Some(&profile));
        assert_eq!(
            ModelLibrary::new(vec![descriptor], vec![profile.clone(), profile]),
            Err("model profile ids must be unique")
        );
    }
}
