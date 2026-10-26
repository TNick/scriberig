//! Stores provider adapters from one atomically published extension generation.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use rpi_ai::{Model, Provider};
use rpi_extensions::GenerationCandidate;
use rpi_model_runtime::ModelCatalog;

/// Shared provider adapter type stored with the published generation.
pub(super) type ProviderAdapter = Arc<dyn Provider>;

/// Publishes staged provider, extension-session, and catalog state together.
///
/// # Parameters
/// - `registry`: Shared provider adapter map.
/// - `extension_generation`: Upstream extension generation slot.
/// - `model_catalog`: Upstream model catalog paired with the session.
/// - `stage`: Host operation returning the validated candidate, model rows,
///   and provider adapters.
///
/// # Returns
/// The published generation and the retired generation lease.
///
/// # Errors
/// Returns an upstream staging or generation publication failure.
pub(super) fn publish_provider_extension_generation<E, F>(
    registry: &ProviderAdapterRegistry,
    extension_generation: &rpi_extensions::ExtensionGenerationSlot,
    model_catalog: &ModelCatalog,
    stage: F,
) -> Result<rpi_extensions::PublishedGeneration, rpi_model_runtime::ExtensionCatalogRefreshError<E>>
where
    F: FnOnce() -> Result<(GenerationCandidate, Vec<Model>, Vec<ProviderAdapter>), E>,
{
    let (candidate, models, providers) =
        stage().map_err(rpi_model_runtime::ExtensionCatalogRefreshError::Stage)?;
    let mut live_providers = registry.lock_for_publication();
    let published = rpi_model_runtime::refresh_extension_catalog_generation(
        extension_generation,
        model_catalog,
        || Ok::<_, std::convert::Infallible>((candidate, models)),
    )
    .map_err(map_publication_error)?;
    live_providers.replace(providers);
    Ok(published)
}

impl ProviderAdapterRegistry {
    /// Locks provider lookups until the catalog generation is published.
    ///
    /// # Returns
    /// Write access to the active provider adapter map.
    pub(super) fn lock_for_publication(&self) -> ProviderAdapterRegistryWriteGuard<'_> {
        ProviderAdapterRegistryWriteGuard(
            self.inner
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    /// Resolves one provider adapter from the current published set.
    ///
    /// # Parameters
    /// - `provider_id`: Exact provider identity.
    ///
    /// # Returns
    /// A cloned provider adapter, when registered.
    pub(super) fn get(&self, provider_id: &str) -> Option<ProviderAdapter> {
        self.inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(provider_id)
            .cloned()
    }

    /// Projects the active provider identities while blocking generation replacement.
    ///
    /// The projection holds the provider read lock while it runs, so a caller
    /// may combine these ids with the paired extension/catalog snapshot without
    /// observing the provider map between publication steps.
    ///
    /// # Parameters
    /// - `project`: Projection evaluated with the active provider ids.
    ///
    /// # Returns
    /// The value returned by `project`.
    pub(super) fn with_active_ids<R>(&self, project: impl FnOnce(Vec<String>) -> R) -> R {
        let providers = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        project(providers.keys().cloned().collect())
    }
}

/// Shared lock protecting active provider adapters.
#[derive(Clone, Default)]
pub(super) struct ProviderAdapterRegistry {
    /// Adapter map paired with the active extension generation.
    inner: Arc<RwLock<BTreeMap<String, ProviderAdapter>>>,
}

/// Write lock held during extension/catalog/provider publication.
pub(super) struct ProviderAdapterRegistryWriteGuard<'a>(
    /// Adapter map locked against provider readers.
    RwLockWriteGuard<'a, BTreeMap<String, ProviderAdapter>>,
);

impl ProviderAdapterRegistryWriteGuard<'_> {
    /// Replaces the provider map with adapters staged for the new generation.
    ///
    /// # Parameters
    /// - `providers`: Host-authorized adapters for the candidate generation.
    pub(super) fn replace(&mut self, providers: Vec<ProviderAdapter>) {
        *self.0 = providers
            .into_iter()
            .map(|provider| (provider.id().to_owned(), provider))
            .collect();
    }
}

/// Converts the upstream infallible stage error and propagates publication errors.
///
/// # Parameters
/// - `error`: Failure returned by the upstream publication gate.
///
/// # Returns
/// The corresponding provider-extension refresh error.
fn map_publication_error<E>(
    error: rpi_model_runtime::ExtensionCatalogRefreshError<std::convert::Infallible>,
) -> rpi_model_runtime::ExtensionCatalogRefreshError<E> {
    match error {
        rpi_model_runtime::ExtensionCatalogRefreshError::Stage(never) => match never {},
        rpi_model_runtime::ExtensionCatalogRefreshError::Publish(error) => {
            rpi_model_runtime::ExtensionCatalogRefreshError::Publish(error)
        }
    }
}
