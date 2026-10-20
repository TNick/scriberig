//! Provides the shared constructor for local child runtime composition.

#[cfg(test)]
use super::LocalChildExecutor;
#[cfg(test)]
use scriberig_agent_runtime::cache::ModelCache;
#[cfg(test)]
use scriberig_agent_runtime::provider::ProviderAdapter;
#[cfg(test)]
use scriberig_app::{ApplicationBackend, ApplicationService};
#[cfg(test)]
use scriberig_store_sqlite::SqliteStore;

#[cfg(test)]
impl<'a, B, P> LocalChildExecutor<'a, B, P>
where
    B: ApplicationBackend,
    P: ProviderAdapter + Clone,
{
    /// Creates a test child executor without a selected thinking level.
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
    #[cfg(test)]
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
            provider_id: None,
            provider_profile: None,
            thinking_level: None,
            cache,
            project_store,
        }
    }
}
