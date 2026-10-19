//! Builds safe model-library values from trusted composition configuration.

use super::model::{
    ModelCapabilities, ModelDescriptor, ModelId, ModelLibrary, ModelProfile,
    ThinkingLevelDescriptor, ThinkingLevelId,
};

#[derive(serde::Deserialize)]
/// Configuration envelope for curated model entries.
struct CatalogInput {
    /// Models available through one configured provider connection.
    models: Vec<ModelInput>,
}

#[derive(serde::Deserialize)]
/// One provider model and its model-specific thinking capabilities.
struct ModelInput {
    /// Stable Scriberig model identity.
    id: String,
    /// Safe user-facing model name.
    display_name: String,
    /// Provider-native model identifier.
    provider_model_id: String,
    /// Stable provider-family identity, when it differs from the default.
    provider_profile: Option<String>,
    /// Credential-free connection identity used by this model profile.
    connection: Option<String>,
    /// Thinking levels supported by this model.
    #[serde(default)]
    thinking_levels: Vec<ThinkingLevelInput>,
    /// Optional model default thinking level.
    default_thinking_level: Option<String>,
}

#[derive(serde::Deserialize)]
/// One advertised model-specific thinking level.
struct ThinkingLevelInput {
    /// Stable level identity interpreted by the adapter.
    id: String,
    /// Safe user-facing label.
    label: String,
    /// Optional safe description.
    description: Option<String>,
    /// Optional presentation order.
    order: Option<u32>,
}

/// Builds a validated model library from trusted non-secret catalog JSON.
///
/// Provider-specific `provider_parameters` fields in the input are ignored by
/// the application projection and must be consumed only by the runtime adapter.
/// When the JSON is absent, a single fallback model is exposed without thinking
/// levels. Every configured model gets one enabled profile on the local
/// connection.
///
/// - `json`: Optional catalog object with a `models` array.
/// - `fallback_model`: Runtime model exposed when no catalog is configured.
///
/// Safe, validated model descriptors and selectable profiles.
///
/// Returns an error for malformed or internally inconsistent catalog data.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn model_library_from_json(
    json: Option<&str>,
    fallback_model: &str,
) -> Result<ModelLibrary, &'static str> {
    let input = match json {
        Some(value) => serde_json::from_str::<CatalogInput>(value)
            .map_err(|_| "model catalog JSON is invalid")?,
        None => CatalogInput {
            models: vec![ModelInput {
                id: fallback_model.to_owned(),
                display_name: fallback_model.to_owned(),
                provider_model_id: fallback_model.to_owned(),
                provider_profile: None,
                connection: None,
                thinking_levels: Vec::new(),
                default_thinking_level: None,
            }],
        },
    };
    let mut models = Vec::new();
    let mut profiles = Vec::new();
    for model_input in input.models {
        let model_id = ModelId::new(model_input.id)?;
        let levels = model_input
            .thinking_levels
            .into_iter()
            .enumerate()
            .map(|(index, level)| {
                Ok(ThinkingLevelDescriptor {
                    id: ThinkingLevelId::new(level.id)?,
                    label: level.label,
                    description: level.description,
                    order: level
                        .order
                        .unwrap_or(u32::try_from(index + 1).unwrap_or(u32::MAX)),
                })
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        let default_level = model_input
            .default_thinking_level
            .map(ThinkingLevelId::new)
            .transpose()?;
        let connection = model_input.connection.unwrap_or_else(|| "local".to_owned());
        let profile_id = format!("{}-profile", model_id.as_str());
        models.push(ModelDescriptor {
            id: model_id.clone(),
            display_name: model_input.display_name.clone(),
            provider_profile: model_input
                .provider_profile
                .unwrap_or_else(|| "local".to_owned()),
            provider_model_id: model_input.provider_model_id,
            capabilities: ModelCapabilities {
                streaming: true,
                tool_calls: true,
                structured_output: false,
            },
            thinking_levels: levels,
            default_thinking_level: default_level.clone(),
        });
        profiles.push(ModelProfile {
            id: profile_id,
            display_name: model_input.display_name,
            model: model_id,
            connection,
            credential: None,
            default_thinking_level: default_level,
            enabled: true,
        });
    }
    ModelLibrary::new(models, profiles)
}

#[cfg(test)]
mod tests {
    use super::model_library_from_json;

    /// Excludes private provider wire data from returned public descriptors.
    #[test]
    fn configured_catalog_projects_safe_model_specific_levels() {
        let library = model_library_from_json(
            Some(
                r#"{"models":[{"id":"one","display_name":"One","provider_model_id":"wire-one","thinking_levels":[{"id":"deep","label":"Deep","provider_parameters":{"secret":"fake-secret-marker"}}]},{"id":"two","display_name":"Two","provider_model_id":"wire-two","thinking_levels":[{"id":"deep","label":"Deep"}]}]}"#,
            ),
            "fallback",
        )
        .expect("valid catalog");
        let serialized = serde_json::to_string(&library).expect("serialize library");

        assert_eq!(library.models.len(), 2);
        assert_eq!(library.profiles.len(), 2);
        assert_eq!(library.models[0].thinking_levels[0].id.as_str(), "deep");
        assert_eq!(library.models[1].thinking_levels[0].id.as_str(), "deep");
        assert!(!serialized.contains("provider_parameters"));
        assert!(!serialized.contains("fake-secret-marker"));
    }

    /// Preserves the configured provider family and safe connection identity.
    #[test]
    fn configured_catalog_preserves_provider_connection_assignment() {
        let library = model_library_from_json(
            Some(
                r#"{"models":[{"id":"alternate","display_name":"Alternate","provider_model_id":"wire-alternate","provider_profile":"openai_compatible","connection":"alternate-connection"}]}"#,
            ),
            "fallback",
        )
        .expect("valid provider connection catalog");

        assert_eq!(library.models[0].provider_profile, "openai_compatible");
        assert_eq!(library.profiles[0].connection, "alternate-connection");
        let serialized = serde_json::to_string(&library).expect("serialize library");
        assert!(!serialized.contains("api_key"));
    }

    /// Exposes the configured fallback when catalog JSON is absent.
    #[test]
    fn absent_catalog_exposes_fallback_model_without_levels() {
        let library = model_library_from_json(None, "runtime-default").expect("fallback");

        assert_eq!(library.models[0].provider_model_id, "runtime-default");
        assert_eq!(library.models[0].thinking_levels, Vec::new());
        assert_eq!(library.profiles[0].model.as_str(), "runtime-default");
    }
}
