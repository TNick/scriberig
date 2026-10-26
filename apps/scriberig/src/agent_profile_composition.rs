//! Composes semantic profile storage over the shared SQLite knowledge store.

use std::sync::Arc;

use scriberig_app::{AgentProfileProvider, AgentProfileService};
use scriberig_core::RootName;
use scriberig_store_sqlite::SqliteStore;

/// Creates the shared application profile provider for an integrated store.
///
/// # Parameters
/// - `store`: SQLite universal revision, snapshot, and root store.
///
/// # Returns
/// A provider reading exact profiles from the durable profile root.
pub(crate) fn provider(store: Arc<SqliteStore>) -> Arc<dyn AgentProfileProvider> {
    let root = RootName::new("agent-profiles").expect("fixed profile root is valid");
    Arc::new(AgentProfileService::new(store, root))
}
