//! Provides the shared constructor for local child runtime composition.

use scriberig_agent_runtime::cache::ModelCache;
use scriberig_agent_runtime::provider::ProviderAdapter;
use scriberig_app::{ApplicationBackend, ApplicationService};
use scriberig_store_sqlite::SqliteStore;

use super::LocalChildExecutor;

impl<'a, B, P> LocalChildExecutor<'a, B, P>
where
    B: ApplicationBackend,
    P: ProviderAdapter + Clone,
{
    /// Creates a child executor without a session-selected thinking level.
    ///
    /// - `service`: Application boundary owning session and operation state.
    /// - `provider`: Cloneable provider adapter for child requests.
    /// - `model`: Provider-native model identifier.
    /// - `cache`: Shared response cache for the execution tree.
    /// - `project_store`: Durable project store used by child tools.
    ///
    /// A child executor using the supplied runtime model and dependencies.
    /// # Returns
    /// The operation result.
    pub(crate) fn new(
        service: &'a ApplicationService<B>,
        provider: P,
        model: String,
        cache: ModelCache,
        project_store: std::sync::Arc<SqliteStore>,
    ) -> Self {
        Self {
            service,
            operation: None,
            provider,
            model,
            thinking_level: None,
            cache,
            project_store,
        }
    }
}
