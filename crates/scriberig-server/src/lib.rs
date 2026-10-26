//! Adapts remote protocol requests and events onto shared Scriberig application operations.

pub mod development;
pub mod dispatch;
pub mod http;
pub mod operation;
pub mod server;
pub mod subscription;
pub mod transport;

pub use development::{DevelopmentFixture, ensure_fixture};
pub use dispatch::Dispatcher;
pub use http::{AgentExecutionHandler, InProcessProjectTransport, ProviderReadinessHandler};
pub use server::ApplicationService;
pub use subscription::SubscriptionSpec;
pub use transport::InProcessTransport;

use std::error::Error;
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;

/// Backend composed by the local SQLite-backed HTTP server.
pub type LocalApplicationBackend = scriberig_app::ApplicationBackendComposition<
    scriberig_app::StoreInspectionBackend<Arc<scriberig_store_sqlite::SqliteStore>>,
    scriberig_app::StoreOperationProvider<Arc<scriberig_store_sqlite::SqliteStore>>,
>;

/// Application service composed by the local SQLite-backed HTTP server.
pub type LocalApplicationService = scriberig_app::ApplicationService<LocalApplicationBackend>;

/// Trusted native runtime for advancing a durable plan lifecycle request.
pub type PlanLifecycleHandler = dyn Fn(
        &LocalApplicationService,
        &scriberig_app::ApplicationContext,
        &scriberig_app::PlanLifecycleRequest,
        scriberig_app::PlanLifecycleAction,
    ) -> Result<scriberig_session::Plan, scriberig_app::ApplicationError>
    + Send
    + Sync;

/// Serve the development health endpoint over HTTP.
///
/// - `host`: Interface address to bind.
/// - `port`: TCP port to bind.
/// - `database_path`: SQLite file to initialize before accepting requests.
///
/// Unit after the listener stops accepting connections.
///
/// Returns an error when the database or listener cannot be initialized.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn serve(
    host: &str,
    port: u16,
    database_path: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    serve_with_origin(host, port, database_path, None, &[])
}

/// Serves the API with an optional exact browser origin policy.
///
/// - `host`: Interface address to bind.
/// - `port`: TCP port to bind.
/// - `database_path`: SQLite file to initialize before accepting requests.
/// - `allowed_origin`: Exact browser origin allowed for CORS responses.
/// - `project_roots`: Filesystem roots allowed for project onboarding.
///
/// Unit after the listener stops accepting connections.
///
/// Returns an error for invalid origin configuration, database failure, or
/// listener initialization failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn serve_with_origin(
    host: &str,
    port: u16,
    database_path: &Path,
    allowed_origin: Option<&str>,
    project_roots: &[std::path::PathBuf],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    serve_with_origin_and_plan_lifecycle(
        host,
        port,
        database_path,
        allowed_origin,
        project_roots,
        None,
    )
}

/// Serves the API with a trusted provider-backed plan execution runtime.
///
/// - `host`: Interface address to bind.
/// - `port`: TCP port to bind.
/// - `database_path`: SQLite file to initialize before accepting requests.
/// - `allowed_origin`: Exact browser origin allowed for CORS responses.
/// - `project_roots`: Filesystem roots allowed for project onboarding.
/// - `plan_lifecycle_handler`: Optional runtime that executes lifecycle requests.
///
/// Unit after the listener stops accepting connections.
///
/// Returns an error for invalid origin configuration, database failure, or
/// listener initialization failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn serve_with_origin_and_plan_lifecycle(
    host: &str,
    port: u16,
    database_path: &Path,
    allowed_origin: Option<&str>,
    project_roots: &[std::path::PathBuf],
    plan_lifecycle_handler: Option<Arc<PlanLifecycleHandler>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    serve_with_model_catalog_and_plan_lifecycle(
        host,
        port,
        database_path,
        allowed_origin,
        project_roots,
        plan_lifecycle_handler,
        ServerModelCatalog {
            provider_extension_runtime: scriberig_app::ProviderExtensionRuntime::new(),
            provider_readiness_handler: None,
        },
    )
}

/// Carries the upstream model authority for shared application composition.
///
/// # Public members
/// - `provider_extension_runtime`: Shared catalog and extension generation.
/// - `provider_readiness_handler`: Optional live host-authorized credential
///   readiness projection.
///
/// # Private members
/// None.
pub struct ServerModelCatalog {
    /// Shared catalog and extension publication generation.
    pub provider_extension_runtime: scriberig_app::ProviderExtensionRuntime,
    /// Projects current safe credential readiness on model-library reads.
    pub provider_readiness_handler: Option<Arc<ProviderReadinessHandler>>,
}

/// Serves the API with a host-composed upstream model catalog.
///
/// - `host`: Interface address to bind.
/// - `port`: TCP port to bind.
/// - `database_path`: SQLite file to initialize before accepting requests.
/// - `allowed_origin`: Exact browser origin allowed for CORS responses.
/// - `project_roots`: Filesystem roots allowed for project onboarding.
/// - `plan_lifecycle_handler`: Optional runtime that executes lifecycle requests.
/// - `model_catalog`: Upstream selection authority.
/// - `provider_readiness_handler`: Optional live host-authorized readiness
///   projection for model-library responses.
///
/// Unit after the listener stops accepting connections.
///
/// Returns an error for invalid origin configuration, database failure, or
/// listener initialization failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn serve_with_model_catalog_and_plan_lifecycle(
    host: &str,
    port: u16,
    database_path: &Path,
    allowed_origin: Option<&str>,
    project_roots: &[std::path::PathBuf],
    plan_lifecycle_handler: Option<Arc<PlanLifecycleHandler>>,
    model_catalog: ServerModelCatalog,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    serve_with_projection_style(
        host,
        port,
        database_path,
        allowed_origin,
        project_roots,
        plan_lifecycle_handler,
        model_catalog,
        scriberig_app::ProjectionStyleConfiguration::default(),
    )
}

/// Serves a local API with an explicit projection style configuration.
///
/// - `host`: Listener host.
/// - `port`: Listener port.
/// - `database_path`: SQLite database path.
/// - `allowed_origin`: Exact CORS origin, when allowed.
/// - `project_roots`: Host-authorized project roots for file operations.
/// - `plan_lifecycle_handler`: Optional host-authorized plan lifecycle handler.
/// - `model_catalog`: Host-authorized model catalog handle.
/// - `projection_style`: Domain style adapters and settings the surfaces share.
///
/// # Returns
/// Unit after the listener stops accepting connections.
///
/// # Errors
/// Returns an error for invalid origin configuration, database failure, or
/// listener initialization failure.
#[allow(clippy::too_many_arguments)] // Keeps one serve entry point beside the existing variants.
pub fn serve_with_projection_style(
    host: &str,
    port: u16,
    database_path: &Path,
    allowed_origin: Option<&str>,
    project_roots: &[std::path::PathBuf],
    plan_lifecycle_handler: Option<Arc<PlanLifecycleHandler>>,
    model_catalog: ServerModelCatalog,
    projection_style: scriberig_app::ProjectionStyleConfiguration,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    serve_with_agent_execution(
        host,
        port,
        database_path,
        allowed_origin,
        project_roots,
        None,
        plan_lifecycle_handler,
        model_catalog,
        projection_style,
    )
}

/// Serves the API with trusted agent and plan execution runtimes.
///
/// - `host`: Listener host.
/// - `port`: Listener port.
/// - `database_path`: SQLite database path.
/// - `allowed_origin`: Exact CORS origin, when allowed.
/// - `project_roots`: Host-authorized project roots.
/// - `agent_execution_handler`: Optional host runtime for accepted agent work.
/// - `plan_lifecycle_handler`: Optional host runtime for plan execution.
/// - `model_catalog`: Host-authorized upstream model catalog.
/// - `projection_style`: Shared domain projection configuration.
///
/// Unit after the listener stops accepting connections.
///
/// Returns an error when configuration, storage, or listener setup fails.
///
/// # Returns
/// Success after the server listener exits cleanly.
///
/// # Errors
/// Returns an error when configuration, storage, or listener setup fails.
///
/// # Panics
/// Panics only if the compile-time-valid `agent-profiles` root name becomes invalid.
#[allow(clippy::too_many_arguments)]
pub fn serve_with_agent_execution(
    host: &str,
    port: u16,
    database_path: &Path,
    allowed_origin: Option<&str>,
    project_roots: &[std::path::PathBuf],
    agent_execution_handler: Option<Arc<AgentExecutionHandler<LocalApplicationService>>>,
    plan_lifecycle_handler: Option<Arc<PlanLifecycleHandler>>,
    model_catalog: ServerModelCatalog,
    projection_style: scriberig_app::ProjectionStyleConfiguration,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if allowed_origin.is_some_and(|origin| origin.is_empty() || origin == "*") {
        return Err("allowed origin must be a non-empty exact origin".into());
    }
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(database_path)?);
    let fixture = ensure_fixture(&store).map_err(std::io::Error::other)?;
    let read = scriberig_app::StoreInspectionBackend::new(Arc::clone(&store));
    let operations = scriberig_app::StoreOperationProvider::new(Arc::clone(&store));
    let backend = scriberig_app::ApplicationBackendComposition::new(read, operations);
    let session_store = Arc::new(scriberig_session_sqlite::SqliteSessionStore::open(
        database_path,
    )?);
    let service = scriberig_app::ApplicationService::new(backend)
        .with_session_store(session_store)
        .with_agent_profile_provider(Arc::new(scriberig_app::AgentProfileService::new(
            Arc::clone(&store),
            scriberig_core::RootName::new("agent-profiles").expect("fixed profile root is valid"),
        )))
        .with_provider_extension_runtime(model_catalog.provider_extension_runtime)
        .with_projection_style_configuration(projection_style);
    let state = http::HttpState {
        service,
        principal: scriberig_app::Principal::new("local-user"),
        fixture,
        allowed_origin: allowed_origin.map(ToOwned::to_owned),
        project_store: Some(Arc::clone(&store)),
        allowed_project_roots: canonical_project_roots(project_roots)?,
        agent_execution_handler,
        plan_lifecycle_handler,
        provider_readiness_handler: model_catalog.provider_readiness_handler,
    };
    let listener = TcpListener::bind((host, port))?;
    println!("Scriberig API listening on http://{host}:{port}");
    println!("SQLite database: {}", database_path.display());

    let state = std::sync::Arc::new(state);
    for connection in listener.incoming() {
        match connection {
            Ok(connection) => {
                let state = std::sync::Arc::clone(&state);
                std::thread::spawn(move || {
                    if let Err(error) = http::handle_connection(connection, &state) {
                        eprintln!("Scriberig API client connection failed: {error}");
                    }
                });
            }
            Err(error) => {
                eprintln!("Scriberig API accept failed: {error}");
            }
        }
    }
    Ok(())
}

/// Resolves configured project roots, defaulting to the current directory.
///
/// - `roots`: Operator-provided project roots.
///
/// Canonical roots used by the HTTP path policy.
///
/// Returns an error when a configured root cannot be resolved.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn canonical_project_roots(
    roots: &[std::path::PathBuf],
) -> Result<Vec<std::path::PathBuf>, Box<dyn Error + Send + Sync>> {
    let roots = if roots.is_empty() {
        vec![std::env::current_dir()?]
    } else {
        roots.to_vec()
    };
    roots
        .into_iter()
        .map(|root| std::fs::canonicalize(root).map_err(Into::into))
        .collect()
}
