//! Carries one upstream model-property edit into the resolved provider request.

use super::*;
use rpi_ai::providers::faux::FauxScript;
use scriberig_agent_runtime::continuation::EphemeralContinuationOwner;
use scriberig_tui::service::TuiService;

/// Provider identity of the file-backed fixture row.
const FIXTURE_PROVIDER: &str = "local";
/// Model identity of the file-backed fixture row.
const FIXTURE_MODEL: &str = "property-model";
/// Upstream schema-declared property edited through the property surface.
const FIXTURE_PROPERTY: &str = "region";
/// Provider-defined property with no product-specific schema handling.
const FUTURE_PROPERTY: &str = "future_vendor_tuning_v3";

/// Builds upstream configuration declaring one editor-safe provider property.
///
/// # Returns
/// A file-backed upstream model configuration with an empty property value.
fn property_fixture_config() -> rpi_model_runtime::ModelsConfig {
    let source = serde_json::json!({
        "providers": {
            FIXTURE_PROVIDER: {
                "api": "openai-completions",
                "baseUrl": "https://properties.example.test",
                "models": [{
                    "id": FIXTURE_MODEL,
                    "providerOptionsSchema": {
                        "version": 1,
                        "schema": {
                            "type": "object",
                            "properties": {
                                FIXTURE_PROPERTY: {
                                    "type": "string",
                                    "enum": ["east", "west"]
                            },
                            FUTURE_PROPERTY: {
                                "type": "integer",
                                "minimum": 1
                                }
                            },
                            "additionalProperties": false
                        }
                    }
                }]
            }
        }
    });
    serde_json::from_value(source).expect("upstream property model configuration")
}

/// Builds the live upstream catalog the host publishes for a file-backed config.
///
/// # Parameters
/// - `config`: Upstream configuration to publish.
///
/// # Returns
/// A shared live catalog handle observing the published rows.
///
/// # Panics
/// Panics when upstream cannot publish the fixture configuration.
fn live_catalog(config: &rpi_model_runtime::ModelsConfig) -> rpi_model_runtime::ModelCatalog {
    let mut config = config.clone();
    let endpoint_config = config.clone();
    let resolver = crate::native_tui_agent::provider_config::catalog::EnvironmentResolver;
    let catalog = rpi_model_runtime::ModelCatalog::new();
    catalog
        .refresh_from_configuration(&mut config, &[], &resolver, |models| {
            rpi_model_runtime::resolve_model_endpoints(models, &endpoint_config, &resolver)
                .map_err(|_| {
                    scriberig_agent_runtime::host_configuration::HostConfigurationError::Configuration(
                        "configured model endpoint could not be resolved".to_owned(),
                    )
                })
        })
        .expect("live upstream model catalog");
    catalog
}

#[expect(
    clippy::too_many_lines,
    reason = "One scenario must hold the config edit, live catalog refresh, provider attempt, and durable request assertions together."
)]
#[test]
/// Drives one custom-model property from the upstream property surface into the
/// durable resolved provider request.
///
/// The submitted value is exactly what the upstream `rpi_tui::PropertyEditor`
/// seam produces; the TUI property adapter converts the chosen text into typed
/// JSON before calling `TuiService::set_model_property`. The edit travels the
/// production authorization hop, the upstream schema validator, the transactional
/// `ModelsConfigFile` writer, and the live catalog refresh, and the assertion
/// targets the resolved request the runtime captured for that attempt, which is
/// the same durable provenance the TUI model request detail surface renders.
fn upstream_property_edit_reaches_the_resolved_provider_request() {
    let root = std::env::temp_dir().join(format!("scriberig-property-request-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("fixture root");
    let config_path = root.join("models.json");
    rpi_model_runtime::ModelsConfigFile::save(&config_path, &property_fixture_config())
        .expect("upstream model configuration file");

    let project_store = Arc::new(SqliteStore::open(root.join("project.sqlite")).expect("project"));
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&project_store)),
        StoreOperationProvider::new(Arc::clone(&project_store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(root.join("session.sqlite"))
            .expect("session store"),
    );
    let session_repository: Arc<dyn scriberig_session::SessionStore> = session_store.clone();
    let model_catalog = live_catalog(&property_fixture_config());
    let provider_extension_runtime =
        scriberig_app::ProviderExtensionRuntime::with_model_catalog(model_catalog.clone());
    let service = ApplicationService::new(backend)
        .with_session_store(session_repository)
        .with_provider_extension_runtime(provider_extension_runtime.clone());
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::from([(
            "local".to_owned(),
            NativeScenario::Faux(FauxScript::new().with_text("unused")),
        )]),
        model: FIXTURE_MODEL.to_owned(),
        upstream_credentials: None,
        upstream_enabled: false,
        model_catalog: model_catalog.clone(),
        provider_extension_runtime: provider_extension_runtime.clone(),
        plugin_command_worker: std::sync::Mutex::new(None),
        extension_component_mailbox: rpi_extensions::ExtensionStatusMailbox::new(),
        provider_lifecycle:
            crate::native_tui_agent::provider_readiness::ProviderLifecycleState::new(),
        model_config: std::sync::Mutex::new((property_fixture_config(), Some(config_path.clone()))),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
        continuations: EphemeralContinuationOwner::default(),
    };
    let adapter = ApplicationOperationAdapter::new(service).with_agent_runner(runner);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_start").expect("agent grant"),
            CapabilityId::new("agent_continue").expect("continuation grant"),
            CapabilityId::new("agent_model_config_edit").expect("model config grant"),
            CapabilityId::new("inspect.entity").expect("inspection grant"),
            CapabilityId::new("session_branch_tree").expect("session grant"),
        ]),
        ..ApplicationContext::empty()
    };

    TuiService::set_model_property(
        &adapter,
        &context,
        FIXTURE_PROVIDER,
        FIXTURE_MODEL,
        FIXTURE_PROPERTY,
        serde_json::json!("west"),
    )
    .expect("authorized upstream property edit");
    TuiService::set_model_property(
        &adapter,
        &context,
        FIXTURE_PROVIDER,
        FIXTURE_MODEL,
        FUTURE_PROPERTY,
        serde_json::json!(7),
    )
    .expect("provider-defined future property edit");

    let persisted =
        rpi_model_runtime::ModelsConfigFile::load(&config_path).expect("persisted configuration");
    assert_eq!(
        persisted.providers[FIXTURE_PROVIDER].models[0].provider_options[FIXTURE_PROPERTY],
        serde_json::json!("west"),
        "upstream validation must persist the edited property"
    );
    assert_eq!(
        persisted.providers[FIXTURE_PROVIDER].models[0].provider_options[FUTURE_PROPERTY],
        serde_json::json!(7),
        "an unrecognized provider property key must persist without product-specific handling"
    );
    let published = model_catalog
        .snapshot()
        .models
        .into_iter()
        .find(|model| model.provider == FIXTURE_PROVIDER && model.id == FIXTURE_MODEL)
        .expect("published catalog model");
    assert_eq!(
        published.provider_options[FIXTURE_PROPERTY],
        serde_json::json!("west"),
        "the live catalog row handed to provider construction must carry the property"
    );
    assert_eq!(
        published.provider_options[FUTURE_PROPERTY],
        serde_json::json!(7),
        "the live catalog must retain provider-defined future properties"
    );
    let paired = provider_extension_runtime.extension_catalog_snapshot();
    assert_eq!(
        paired.extension.catalog_revision(),
        Some(paired.catalog.revision)
    );
    assert_eq!(paired.catalog.models, model_catalog.snapshot().models);

    let mut state = TuiState::default();
    state.conversation_composer.draft = "Use the configured region.".to_owned();
    let submitted = reduce(&mut state, Action::InputSubmitted);
    let [Effect::StartNewConversation { prompt, .. }] = submitted.as_slice() else {
        panic!("first message should use the shared session start");
    };
    let started = execute_effect(
        &adapter,
        &context,
        Effect::StartNewConversation {
            prompt: prompt.clone(),
            selection: None,
        },
    );
    let refresh = reduce(&mut state, started);
    let session = match refresh.as_slice() {
        [Effect::ReadSession { session, .. }] => *session,
        other => panic!("the provider attempt should refresh its conversation: {other:?}"),
    };

    let durable = session_store
        .load(session)
        .expect("load persisted session")
        .expect("conversation session exists");
    let root_branch = durable.branches.first().expect("root branch");
    let invocation = root_branch
        .model_invocations
        .last()
        .expect("resolved provider attempt is durable");
    let request = invocation
        .request
        .as_ref()
        .expect("prepared upstream request is retained");
    assert_eq!(request.model, FIXTURE_MODEL);
    let execution = request
        .model_execution_configuration
        .as_ref()
        .expect("upstream execution configuration for the resolved request");
    assert_eq!(
        execution["providerOptions"][FIXTURE_PROPERTY],
        serde_json::json!("west"),
        "the resolved provider request must carry the edited upstream property"
    );
    assert_eq!(
        execution["providerOptions"][FUTURE_PROPERTY],
        serde_json::json!(7),
        "the prepared request must retain provider-defined future properties"
    );

    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}
