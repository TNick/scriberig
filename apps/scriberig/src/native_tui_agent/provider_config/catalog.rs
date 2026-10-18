//! Parses safe model catalog metadata and adapter-only thinking parameters.

use std::collections::BTreeMap;

use scriberig_agent_runtime::provider::ProviderError;
use scriberig_app::{ModelLibrary, ThinkingLevelId};

/// Separates public descriptors from provider-private wire translations.
pub(in crate::native_tui_agent) struct ProviderCatalog {
    /// Safe model descriptors and selectable profiles.
    pub(in crate::native_tui_agent) library: ModelLibrary,
    /// Adapter-only fields keyed by provider model and public level id.
    pub(in crate::native_tui_agent) thinking_parameters:
        Vec<(String, String, BTreeMap<String, serde_json::Value>)>,
}

/// Raw JSON object containing provider model entries.
#[derive(serde::Deserialize)]
struct CatalogInput {
    /// Models advertised by this runtime configuration.
    models: Vec<ModelInput>,
}

/// Raw model descriptor parsed from trusted runtime configuration.
#[derive(serde::Deserialize)]
struct ModelInput {
    /// Provider-native identifier sent to the adapter.
    provider_model_id: String,
    /// Thinking levels advertised by this model.
    #[serde(default)]
    thinking_levels: Vec<ThinkingLevelInput>,
}

/// Raw model-specific thinking capability and private wire mapping.
#[derive(serde::Deserialize)]
struct ThinkingLevelInput {
    /// Stable model-specific capability identity.
    id: String,
    /// Provider-only fields applied to the outgoing request.
    #[serde(default)]
    provider_parameters: BTreeMap<String, serde_json::Value>,
}

/// Loads the native runtime model library from its optional JSON environment.
///
/// `SCRIBE_AGENT_MODEL_CATALOG` accepts an object with a `models` array. Each
/// entry names a stable `id`, display name, provider-native model identifier,
/// and optional model-specific `thinking_levels`. A thinking level may include
/// `provider_parameters`; those fields remain inside the provider adapter.
///
/// # Parameters
/// - `fallback_model`: Configured runtime model when no catalog is supplied.
///
/// # Returns
/// Safe descriptors plus adapter-only thinking translations.
///
/// # Errors
/// Returns a generic configuration error for malformed or inconsistent data.
pub(in crate::native_tui_agent) fn from_environment(
    fallback_model: &str,
) -> Result<ProviderCatalog, ProviderError> {
    let catalog = std::env::var("SCRIBE_AGENT_MODEL_CATALOG").ok();
    from_json(catalog.as_deref(), fallback_model)
}

/// Parses catalog JSON or creates the single configured runtime model.
///
/// # Parameters
/// - `json`: Optional model catalog JSON from trusted runtime configuration.
/// - `fallback_model`: Configured runtime model when JSON is absent.
///
/// # Returns
/// Safe descriptors plus adapter-only thinking translations.
///
/// # Errors
/// Returns a generic configuration error for malformed or inconsistent data.
fn from_json(json: Option<&str>, fallback_model: &str) -> Result<ProviderCatalog, ProviderError> {
    let library = scriberig_app::model_library_from_json(json, fallback_model)
        .map_err(|_| ProviderError::Configuration("invalid model catalog".to_owned()))?;
    let input = match json {
        Some(value) => serde_json::from_str::<CatalogInput>(value)
            .map_err(|_| ProviderError::Configuration("invalid model catalog".to_owned()))?,
        None => CatalogInput {
            models: vec![ModelInput {
                provider_model_id: fallback_model.to_owned(),
                thinking_levels: Vec::new(),
            }],
        },
    };
    let mut thinking_parameters = Vec::new();
    for model_input in input.models {
        for level in model_input.thinking_levels {
            let id = ThinkingLevelId::new(level.id)
                .map_err(|_| ProviderError::Configuration("invalid model catalog".to_owned()))?;
            if !level.provider_parameters.is_empty() {
                thinking_parameters.push((
                    model_input.provider_model_id.clone(),
                    id.as_str().to_owned(),
                    level.provider_parameters,
                ));
            }
        }
    }
    Ok(ProviderCatalog {
        library,
        thinking_parameters,
    })
}

#[cfg(test)]
mod tests {
    use super::from_json;

    /// Keeps duplicate level names scoped to their provider model.
    #[test]
    fn catalog_keeps_model_specific_levels_and_wire_values() {
        let catalog = from_json(
            Some(
                r#"{"models":[{"id":"one","display_name":"One","provider_model_id":"wire-one","thinking_levels":[{"id":"deep","label":"Deep","provider_parameters":{"reasoning_effort":"high"}}]},{"id":"two","display_name":"Two","provider_model_id":"wire-two","thinking_levels":[{"id":"deep","label":"Deep","provider_parameters":{"reasoning_budget":12000}}]}]}"#,
            ),
            "fallback",
        )
        .expect("catalog");

        assert_eq!(catalog.library.models.len(), 2);
        assert_eq!(catalog.library.profiles.len(), 2);
        assert_eq!(catalog.thinking_parameters.len(), 2);
        assert_eq!(catalog.thinking_parameters[0].0, "wire-one");
        assert_eq!(catalog.thinking_parameters[1].0, "wire-two");
    }

    /// Rejects invalid thinking defaults before publishing model metadata.
    #[test]
    fn catalog_rejects_unadvertised_default_level() {
        let result = from_json(
            Some(
                r#"{"models":[{"id":"one","display_name":"One","provider_model_id":"wire-one","default_thinking_level":"missing"}]}"#,
            ),
            "fallback",
        );

        assert!(result.is_err());
    }
}
