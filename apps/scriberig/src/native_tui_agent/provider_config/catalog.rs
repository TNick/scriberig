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
    /// Credential-free connections declared by the trusted runtime catalog.
    pub(in crate::native_tui_agent) connections: Vec<ProviderConnection>,
}

/// Describes one provider adapter using environment variable references only.
pub(in crate::native_tui_agent) struct ProviderConnection {
    /// Safe connection identity referenced by model-library profiles.
    pub(in crate::native_tui_agent) id: String,
    /// Adapter family supported by the native runtime.
    pub(in crate::native_tui_agent) provider: String,
    /// Environment variable containing the endpoint URL.
    pub(in crate::native_tui_agent) base_url_env: String,
    /// Optional environment variable containing a credential.
    pub(in crate::native_tui_agent) api_key_env: Option<String>,
}

/// Raw JSON object containing provider model entries.
#[derive(serde::Deserialize)]
struct CatalogInput {
    /// Models advertised by this runtime configuration.
    models: Vec<ModelInput>,
    /// Named provider connections available to those models.
    #[serde(default)]
    connections: Vec<ConnectionInput>,
}

/// Raw model descriptor parsed from trusted runtime configuration.
#[derive(serde::Deserialize)]
struct ModelInput {
    /// Provider-native identifier sent to the adapter.
    provider_model_id: String,
    /// Adapter connection selected by the safe model profile.
    connection: Option<String>,
    /// Thinking levels advertised by this model.
    #[serde(default)]
    thinking_levels: Vec<ThinkingLevelInput>,
}

/// Raw credential-free provider connection configuration.
#[derive(serde::Deserialize)]
struct ConnectionInput {
    /// Stable profile identity used by model assignments.
    id: String,
    /// Adapter family, currently OpenAI-compatible.
    provider: String,
    /// Environment variable holding the endpoint URL.
    base_url_env: String,
    /// Optional environment variable holding the API credential.
    api_key_env: Option<String>,
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
/// - `fallback_model`: Configured runtime model when no catalog is supplied.
///
/// Safe descriptors plus adapter-only thinking translations.
///
/// Returns a generic configuration error for malformed or inconsistent data.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(in crate::native_tui_agent) fn from_environment(
    fallback_model: &str,
) -> Result<ProviderCatalog, ProviderError> {
    let catalog = std::env::var("SCRIBE_AGENT_MODEL_CATALOG").ok();
    from_json(catalog.as_deref(), fallback_model)
}

/// Parses catalog JSON or creates the single configured runtime model.
///
/// - `json`: Optional model catalog JSON from trusted runtime configuration.
/// - `fallback_model`: Configured runtime model when JSON is absent.
///
/// Safe descriptors plus adapter-only thinking translations.
///
/// Returns a generic configuration error for malformed or inconsistent data.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn from_json(json: Option<&str>, fallback_model: &str) -> Result<ProviderCatalog, ProviderError> {
    let library = scriberig_app::model_library_from_json(json, fallback_model)
        .map_err(|_| ProviderError::Configuration("invalid model catalog".to_owned()))?;
    let input = match json {
        Some(value) => serde_json::from_str::<CatalogInput>(value)
            .map_err(|_| ProviderError::Configuration("invalid model catalog".to_owned()))?,
        None => CatalogInput {
            models: vec![ModelInput {
                provider_model_id: fallback_model.to_owned(),
                connection: None,
                thinking_levels: Vec::new(),
            }],
            connections: Vec::new(),
        },
    };
    let mut thinking_parameters = Vec::new();
    let connections = input
        .connections
        .into_iter()
        .map(|connection| {
            if connection.id.is_empty()
                || connection.provider.is_empty()
                || connection.base_url_env.is_empty()
                || connection
                    .api_key_env
                    .as_ref()
                    .is_some_and(String::is_empty)
            {
                return Err(ProviderError::Configuration(
                    "invalid provider connection".to_owned(),
                ));
            }
            Ok(ProviderConnection {
                id: connection.id,
                provider: connection.provider,
                base_url_env: connection.base_url_env,
                api_key_env: connection.api_key_env,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    for model_input in input.models {
        let connection = model_input.connection.as_deref().unwrap_or("local");
        if connection != "local" && !connections.iter().any(|item| item.id == connection) {
            return Err(ProviderError::Configuration(
                "model references an unknown provider connection".to_owned(),
            ));
        }
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
        connections,
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

    /// Keeps endpoint and credential values indirect in named connection catalogs.
    #[test]
    fn catalog_keeps_provider_credentials_as_environment_references() {
        let catalog = from_json(
            Some(
                r#"{"models":[{"id":"remote","display_name":"Remote","provider_model_id":"wire-remote","provider_profile":"openai_compatible","connection":"remote-a"}],"connections":[{"id":"remote-a","provider":"openai_compatible","base_url_env":"REMOTE_BASE_URL","api_key_env":"REMOTE_API_KEY"}]}"#,
            ),
            "fallback",
        )
        .expect("catalog");

        assert_eq!(catalog.connections.len(), 1);
        assert_eq!(catalog.connections[0].id, "remote-a");
        assert_eq!(
            catalog.connections[0].api_key_env.as_deref(),
            Some("REMOTE_API_KEY")
        );
        assert_eq!(catalog.library.profiles[0].connection, "remote-a");
        let serialized = serde_json::to_string(&catalog.library).expect("safe library");
        assert!(!serialized.contains("REMOTE_API_KEY"));
    }

    /// Rejects models whose provider connection is not configured.
    #[test]
    fn catalog_rejects_unknown_provider_connection() {
        let result = from_json(
            Some(
                r#"{"models":[{"id":"remote","display_name":"Remote","provider_model_id":"wire-remote","connection":"missing"}]}"#,
            ),
            "fallback",
        );

        assert!(result.is_err());
    }
}
