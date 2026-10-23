//! Installs domain projection style adapters in the product composition.

use std::path::Path;
use std::sync::Arc;

use scriberig_app::{KindDispatchStyleAdapter, ProjectionStyleConfiguration};
use scriberig_style_sqlite::SqliteProjectionStyleStore;
use sr_projection::ProjectionStyleAdapter;

/// Builds the projection style configuration for this executable composition.
///
/// Each registered projection kind keeps its own domain style meaning: the Rust
/// domain owns indentation and item spacing, and an unregistered kind is
/// reported as unsupported rather than receiving a generic style. Settings live
/// in the same SQLite file as the rest of the local state, so a preference or a
/// workspace materialization policy survives a process restart.
///
/// - `database_path`: SQLite database path holding local style settings.
///
/// # Returns
/// The configuration to install with
/// `ApplicationService::with_projection_style_configuration`.
///
/// # Errors
/// Returns an I/O error when the style settings table cannot be opened.
pub fn domain_projection_style_configuration(
    database_path: &Path,
) -> Result<ProjectionStyleConfiguration, std::io::Error> {
    let rust_adapter: Arc<dyn ProjectionStyleAdapter> =
        Arc::new(scriberig_domain_rust::RustStyleAdapter);
    let dispatcher = KindDispatchStyleAdapter::new()
        .with_adapter(scriberig_domain_rust::RUST_SOURCE_KIND, rust_adapter);
    let store = SqliteProjectionStyleStore::open(database_path)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(ProjectionStyleConfiguration::new(
        Arc::new(dispatcher),
        Arc::new(store),
    ))
}
