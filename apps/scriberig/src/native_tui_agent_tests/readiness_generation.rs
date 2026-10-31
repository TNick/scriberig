//! Refreshes native model rows and credential readiness from one live catalog.

use super::*;
use scriberig_agent_runtime::continuation::EphemeralContinuationOwner;
use scriberig_tui::service::TuiService;

/// Native TUI adapter composed over the SQLite-backed shared application backend.
type NativeAdapter = ApplicationOperationAdapter<
    ApplicationBackendComposition<
        StoreInspectionBackend<Arc<SqliteStore>>,
        StoreOperationProvider<Arc<SqliteStore>>,
    >,
>;

/// Composes the native TUI adapter over one shared upstream catalog.
///
/// - `root`: Fixture directory holding the durable project store.
/// - `catalog`: Shared live catalog handle used by the service and the runner.
///
/// # Returns
/// The composed adapter and its project store handle for teardown.
fn native_adapter(
    root: &std::path::Path,
    catalog: &rpi_model_runtime::ModelCatalog,
) -> (NativeAdapter, Arc<SqliteStore>) {
    let project_store = Arc::new(
        SqliteStore::open(root.join("project.sqlite")).expect("native readiness project store"),
    );
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&project_store)),
        StoreOperationProvider::new(Arc::clone(&project_store)),
    );
    let provider_extension_runtime =
        scriberig_app::ProviderExtensionRuntime::with_model_catalog(catalog.clone());
    let service = ApplicationService::new(backend)
        .with_provider_extension_runtime(provider_extension_runtime.clone());
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::new(),
        model: "model-before".to_owned(),
        upstream_credentials: None,
        upstream_enabled: false,
        model_catalog: catalog.clone(),
        provider_extension_runtime,
        plugin_command_worker: std::sync::Mutex::new(None),
        extension_component_mailbox: rpi_extensions::ExtensionStatusMailbox::new(),
        provider_lifecycle:
            crate::native_tui_agent::provider_readiness::ProviderLifecycleState::new(),
        model_config: std::sync::Mutex::new((rpi_model_runtime::ModelsConfig::default(), None)),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
        continuations: EphemeralContinuationOwner::default(),
    };
    (
        ApplicationOperationAdapter::new(service).with_agent_runner(runner),
        project_store,
    )
}

/// Trusted local caller allowed to read the model library.
///
/// # Returns
/// The application context used by the readiness fixture.
fn readiness_context() -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_start").expect("agent grant")
        ]),
        ..ApplicationContext::empty()
    }
}

/// Projects the readable provider/model pairs and their safe credential status.
///
/// - `library`: Model-library response to summarize.
///
/// # Returns
/// One `(model id, credential status)` entry per projected row, in order.
fn readiness_rows(
    library: &scriberig_protocol::ModelLibraryResponse,
) -> Vec<(String, scriberig_protocol::ProviderCredentialReadiness)> {
    library
        .provider_readiness
        .iter()
        .map(|row| (row.model_id.clone(), row.credential))
        .collect()
}

/// Verifies one host reload replaces both the model rows and the credential
/// readiness rows returned by the native TUI composition in the same process.
#[test]
fn native_model_library_refreshes_models_and_readiness_after_a_reload() {
    let root = std::env::temp_dir().join(format!("scriberig-readiness-reload-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("fixture root");
    let catalog = model_catalog_from_models(super::plan::upstream_model_catalog(
        "local",
        &["model-before"],
    ));
    let (adapter, project_store) = native_adapter(&root, &catalog);
    let context = readiness_context();

    let before = TuiService::model_library(&adapter, &context).expect("initial model library");
    assert_eq!(
        before
            .models
            .iter()
            .map(|model| model.provider_model_id.as_str())
            .collect::<Vec<_>>(),
        vec!["model-before"]
    );
    assert_eq!(
        readiness_rows(&before),
        vec![(
            "model-before".to_owned(),
            scriberig_protocol::ProviderCredentialReadiness::Missing
        )]
    );

    let reloaded = scriberig_app::reconcile_model_catalog(
        &catalog,
        super::plan::upstream_model_catalog("local", &["model-after"]),
    );

    let after = TuiService::model_library(&adapter, &context).expect("reloaded model library");
    assert!(reloaded.revision() > 0);
    assert_eq!(
        after
            .models
            .iter()
            .map(|model| model.provider_model_id.as_str())
            .collect::<Vec<_>>(),
        vec!["model-after"]
    );
    assert_eq!(
        readiness_rows(&after),
        vec![(
            "model-after".to_owned(),
            scriberig_protocol::ProviderCredentialReadiness::Missing
        )]
    );

    drop(adapter);
    drop(project_store);
    std::fs::remove_dir_all(&root).expect("remove fixture");
}
