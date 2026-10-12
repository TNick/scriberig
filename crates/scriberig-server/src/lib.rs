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
pub use http::InProcessProjectTransport;
pub use server::ApplicationService;
pub use subscription::SubscriptionSpec;
pub use transport::InProcessTransport;

use std::error::Error;
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;

/// Serve the development health endpoint over HTTP.
///
/// # Parameters
/// - `host`: Interface address to bind.
/// - `port`: TCP port to bind.
/// - `database_path`: SQLite file to initialize before accepting requests.
///
/// # Returns
/// Unit after the listener stops accepting connections.
///
/// # Errors
/// Returns an error when the database or listener cannot be initialized.
pub fn serve(
    host: &str,
    port: u16,
    database_path: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    serve_with_origin(host, port, database_path, None, &[])
}

/// Serves the API with an optional exact browser origin policy.
///
/// # Parameters
/// - `host`: Interface address to bind.
/// - `port`: TCP port to bind.
/// - `database_path`: SQLite file to initialize before accepting requests.
/// - `allowed_origin`: Exact browser origin allowed for CORS responses.
/// - `project_roots`: Filesystem roots allowed for project onboarding.
///
/// # Returns
/// Unit after the listener stops accepting connections.
///
/// # Errors
/// Returns an error for invalid origin configuration, database failure, or
/// listener initialization failure.
pub fn serve_with_origin(
    host: &str,
    port: u16,
    database_path: &Path,
    allowed_origin: Option<&str>,
    project_roots: &[std::path::PathBuf],
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
    let service = scriberig_app::ApplicationService::new(backend).with_session_store(session_store);
    let state = http::HttpState {
        service,
        fixture,
        allowed_origin: allowed_origin.map(ToOwned::to_owned),
        project_store: Some(Arc::clone(&store)),
        allowed_project_roots: canonical_project_roots(project_roots)?,
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
/// # Parameters
/// - `roots`: Operator-provided project roots.
///
/// # Returns
/// Canonical roots used by the HTTP path policy.
///
/// # Errors
/// Returns an error when a configured root cannot be resolved.
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
