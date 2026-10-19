//! Defines provider-neutral model library descriptors and thinking capabilities.

use std::collections::BTreeSet;

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
/// Identifies one thinking capability advertised by a specific model.
///
/// # Public members
/// - `new`: Creates a validated thinking identity.
/// - `as_str`: Returns the stable thinking identity text.
///
/// # Private members
/// - The wrapped string stores the stable identity.
pub struct ThinkingLevelId(String);

impl ThinkingLevelId {
    /// Creates a non-empty model-specific thinking identifier.
    ///
    /// - `value`: Stable thinking capability identity.
    ///
    /// The validated thinking identifier.
    ///
    /// Returns an error when the identifier is empty or whitespace-only.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("thinking level id must not be empty");
        }
        Ok(Self(value))
    }

    /// Returns the stable thinking identifier text.
    ///
    /// The identifier value.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        &self.0
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

impl ModelDescriptor {
    /// Validates model identity and thinking capability consistency.
    ///
    /// Returns an error for duplicate or unordered thinking levels, or when
    /// the declared default is not advertised by this model.
    ///
    /// Unit when the descriptor is internally consistent.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn validate(&self) -> Result<(), &'static str> {
        let mut ids = BTreeSet::new();
        let mut previous_order = None;
        for level in &self.thinking_levels {
            if !ids.insert(&level.id) {
                return Err("thinking level ids must be unique within a model");
            }
            if previous_order.is_some_and(|order| order >= level.order) {
                return Err("thinking levels must have increasing order values");
            }
            previous_order = Some(level.order);
        }
        if self
            .default_thinking_level
            .as_ref()
            .is_some_and(|default| !ids.contains(default))
        {
            return Err("default thinking level must be advertised by the model");
        }
        Ok(())
    }
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
    /// Validates profile model and thinking defaults against its descriptor.
    ///
    /// - `model`: Descriptor selected by this profile.
    ///
    /// Returns an error when the profile references another model or declares
    /// a thinking default that the descriptor does not advertise.
    ///
    /// Unit when the profile matches the descriptor.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn validate_for_model(&self, model: &ModelDescriptor) -> Result<(), &'static str> {
        model.validate()?;
        if self.model != model.id {
            return Err("profile model must match the supplied descriptor");
        }
        if self.default_thinking_level.as_ref().is_some_and(|default| {
            !model
                .thinking_levels
                .iter()
                .any(|level| &level.id == default)
        }) {
            return Err("profile thinking default must be advertised by the model");
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
        let first = model("model-a", &["brief", "deliberate"]);
        let second = model("model-b", &["steady", "explore", "deep-dive"]);

        assert!(first.validate().is_ok());
        assert!(second.validate().is_ok());
        assert_eq!(first.thinking_levels.len(), 2);
        assert_eq!(second.thinking_levels.len(), 3);
        assert_eq!(second.thinking_levels[2].id.as_str(), "deep-dive");
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

    /// Verifies unsupported defaults are rejected at descriptor boundaries.
    #[test]
    fn descriptor_rejects_unadvertised_default() {
        let mut descriptor = model("model-a", &["brief"]);
        descriptor.default_thinking_level =
            Some(ThinkingLevelId::new("deep").expect("valid thinking id"));

        assert_eq!(
            descriptor.validate(),
            Err("default thinking level must be advertised by the model")
        );
    }

    /// Verifies profile defaults are constrained by the selected model.
    #[test]
    fn profile_rejects_unadvertised_thinking_default() {
        let descriptor = model("model-a", &["brief"]);
        let profile = ModelProfile {
            id: "work-profile".to_owned(),
            display_name: "Work model".to_owned(),
            model: descriptor.id.clone(),
            connection: "work-endpoint".to_owned(),
            credential: None,
            default_thinking_level: Some(ThinkingLevelId::new("deep").expect("valid thinking id")),
            enabled: true,
        };

        assert_eq!(
            profile.validate_for_model(&descriptor),
            Err("profile thinking default must be advertised by the model")
        );
    }

    /// Verifies library entries reference known models and unique identities.
    #[test]
    fn model_library_validates_profile_references() {
        let descriptor = model("model-a", &["brief"]);
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
