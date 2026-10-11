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
pub use server::ApplicationService;
pub use subscription::SubscriptionSpec;
pub use transport::InProcessTransport;

use std::error::Error;
use std::net::TcpListener;
use std::path::Path;

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
/// Returns an error when the database, listener, or a client connection fails.
pub fn serve(
    host: &str,
    port: u16,
    database_path: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let store = scriberig_store_sqlite::SqliteStore::open(database_path)?;
    let fixture = ensure_fixture(&store).map_err(std::io::Error::other)?;
    let service =
        scriberig_app::ApplicationService::new(scriberig_app::StoreInspectionBackend::new(store));
    let state = http::HttpState { service, fixture };
    let listener = TcpListener::bind((host, port))?;
    println!("Scriberig API listening on http://{host}:{port}");
    println!("SQLite database: {}", database_path.display());

    for connection in listener.incoming() {
        http::handle_connection(connection?, &state)?;
    }
    Ok(())
}
