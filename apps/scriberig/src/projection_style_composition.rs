//! Installs domain projection style adapters in the product composition.

use std::sync::Arc;

use scriberig_app::{
    InMemoryProjectionStyleStore, KindDispatchStyleAdapter, ProjectionStyleConfiguration,
};
use sr_projection::ProjectionStyleAdapter;

/// Builds the projection style configuration for this executable composition.
///
/// Each registered projection kind keeps its own domain style meaning: the Rust
/// domain owns indentation and item spacing, and an unregistered kind is
/// reported as unsupported rather than receiving a generic style. Scope settings
/// are held in memory for now; a durable settings store is separate work.
///
/// # Returns
/// The configuration to install with
/// `ApplicationService::with_projection_style_configuration`.
#[must_use]
pub fn domain_projection_style_configuration() -> ProjectionStyleConfiguration {
    let rust_adapter: Arc<dyn ProjectionStyleAdapter> =
        Arc::new(scriberig_domain_rust::RustStyleAdapter);
    let dispatcher = KindDispatchStyleAdapter::new()
        .with_adapter(scriberig_domain_rust::RUST_SOURCE_KIND, rust_adapter);
    ProjectionStyleConfiguration::new(
        Arc::new(dispatcher),
        Arc::new(InMemoryProjectionStyleStore::new()),
    )
}
