//! Defines the shared provider/model catalog and extension-generation refresh contract.
//!
//! Upstream `rpi-model-runtime` owns the live model catalog, its revision
//! counter, and atomic reconciliation. This module names observed generations
//! and delegates staged extension/catalog publication to upstream. The host
//! closure owns package discovery, root authorization, loading, and validation;
//! `ScribeRig` owns no package registry or second catalog.

use std::sync::Arc;

use rpi_ai::{Model, Provider};
use rpi_extensions::GenerationCandidate;
use rpi_model_runtime::ModelCatalog;

use crate::ApplicationService;

#[path = "provider_lifecycle/provider_adapters.rs"]
mod provider_adapters;
use provider_adapters::{ProviderAdapter, ProviderAdapterRegistry};

/// One immutable generation of the host-owned upstream model catalog.
///
/// The value is upstream's `ModelCatalogSnapshot::revision`; `ScribeRig` keeps no
/// parallel counter. A consumer that cached catalog-derived rows, readiness
/// rows, or a resolved model compares this generation with the live catalog to
/// detect that a reload replaced them.
///
/// # Public members
/// - `revision`: Upstream catalog revision identifying this generation.
///
/// # Private members
/// - `revision`: Exact upstream catalog revision this generation names.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelCatalogGeneration {
    /// Upstream catalog revision identifying this generation.
    revision: u64,
}

/// Shared upstream catalog and extension-generation publication state.
///
/// Clone this handle into every application surface that must observe the same
/// provider rows and loaded extension session generation.
///
/// # Public members
/// - [`ProviderExtensionRuntime::new`]: Creates an empty shared runtime.
/// - [`ProviderExtensionRuntime::with_model_catalog`]: Wraps a shared model catalog.
/// - [`ProviderExtensionRuntime::model_catalog`]: Returns the upstream catalog handle.
/// - [`ProviderExtensionRuntime::refresh_extension_catalog_generation`]: Stages and publishes a pair.
/// - [`ProviderExtensionRuntime::refresh_provider_extension_generation`]: Publishes provider adapters with a pair.
/// - [`ProviderExtensionRuntime::provider`]: Resolves a registered provider adapter.
/// - [`ProviderExtensionRuntime::extension_catalog_snapshot`]: Captures a paired view.
///
/// # Private members
/// - `model_catalog`: Shared upstream model catalog.
/// - `extension_generation`: Shared upstream extension generation slot.
/// - `providers`: Host-owned provider adapters from the same staged generation.
#[derive(Clone)]
pub struct ProviderExtensionRuntime {
    /// Shared upstream model catalog.
    model_catalog: ModelCatalog,
    /// Shared upstream extension generation publication slot.
    extension_generation: rpi_extensions::ExtensionGenerationSlot,
    /// Provider adapters published alongside the active extension generation.
    providers: ProviderAdapterRegistry,
}

impl Default for ProviderExtensionRuntime {
    /// Creates an empty provider-extension runtime.
    ///
    /// # Returns
    /// A runtime with an empty catalog and extension generation.
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderExtensionRuntime {
    /// Creates an empty catalog and extension-generation slot.
    ///
    /// # Returns
    /// A cloneable shared runtime with no extension session loaded.
    #[must_use]
    pub fn new() -> Self {
        Self {
            model_catalog: ModelCatalog::new(),
            extension_generation: rpi_extensions::ExtensionGenerationSlot::empty(),
            providers: ProviderAdapterRegistry::default(),
        }
    }

    /// Creates a runtime around an existing shared upstream model catalog.
    ///
    /// The extension slot starts empty. Product surfaces that share this
    /// runtime should pass its clone rather than separately wrapping the same
    /// catalog, so they share both the rows and their extension generation.
    ///
    /// # Parameters
    /// - `model_catalog`: Shared catalog handle to pair with the new slot.
    ///
    /// # Returns
    /// A runtime that owns the supplied catalog handle and a new empty slot.
    ///
    /// # Panics
    /// Panics if upstream cannot publish the initial catalog generation.
    #[must_use]
    pub fn with_model_catalog(model_catalog: ModelCatalog) -> Self {
        let runtime = Self {
            model_catalog,
            extension_generation: rpi_extensions::ExtensionGenerationSlot::empty(),
            providers: ProviderAdapterRegistry::default(),
        };
        let initial_models = runtime.model_catalog.snapshot().models;
        let candidate =
            GenerationCandidate::from_generation(&runtime.extension_generation.current());
        rpi_model_runtime::publish_extension_catalog_generation(
            &runtime.extension_generation,
            &runtime.model_catalog,
            candidate,
            initial_models,
        )
        .expect("initial catalog generation is prepared");
        runtime
    }

    /// Returns the shared upstream model catalog.
    ///
    /// # Returns
    /// A clone observing the same catalog state.
    #[must_use]
    pub fn model_catalog(&self) -> ModelCatalog {
        self.model_catalog.clone()
    }

    /// Reconciles catalog rows while preserving the current extension generation.
    ///
    /// Changed rows are committed through the shared generation gate with a
    /// candidate cloned from the current session, including its retained worker
    /// resources. Identical rows are a no-op and preserve the current revision.
    ///
    /// # Parameters
    /// - `models`: Complete ordered provider/model rows to publish.
    ///
    /// # Returns
    /// The catalog revision paired with the active extension generation.
    ///
    /// # Panics
    /// Panics if upstream cannot publish a prepared generation.
    #[must_use]
    pub fn reconcile_model_catalog(&self, models: Vec<Model>) -> ModelCatalogGeneration {
        let current_catalog = self.model_catalog.snapshot();
        if current_catalog.models == models {
            return ModelCatalogGeneration::from_revision(current_catalog.revision);
        }
        let current_generation = self.extension_generation.current();
        let candidate = GenerationCandidate::from_generation(&current_generation);
        let published = rpi_model_runtime::publish_extension_catalog_generation(
            &self.extension_generation,
            &self.model_catalog,
            candidate,
            models,
        )
        .expect("catalog reconciliation cannot reject a prepared generation");
        ModelCatalogGeneration::from_revision(
            published
                .current
                .catalog_revision()
                .expect("catalog commit records the paired revision"),
        )
    }

    /// Stages and atomically publishes an extension candidate with its full catalog.
    ///
    /// The host closure owns package discovery, root authorization, loading,
    /// and validation. Staging errors leave the current pair unchanged.
    ///
    /// # Parameters
    /// - `stage`: Host operation returning the validated candidate and all model rows.
    ///
    /// # Returns
    /// The published generation and the retired generation lease.
    ///
    /// # Errors
    /// Returns an upstream staging or publication error.
    pub fn refresh_extension_catalog_generation<E, F>(
        &self,
        stage: F,
    ) -> Result<
        rpi_extensions::PublishedGeneration,
        rpi_model_runtime::ExtensionCatalogRefreshError<E>,
    >
    where
        F: FnOnce() -> Result<(rpi_extensions::GenerationCandidate, Vec<Model>), E>,
    {
        rpi_model_runtime::refresh_extension_catalog_generation(
            &self.extension_generation,
            &self.model_catalog,
            stage,
        )
    }

    /// Stages and atomically publishes provider adapters with their model catalog.
    ///
    /// Provider lookups through this runtime are blocked while the upstream
    /// extension generation and catalog pair is replaced, then observe the
    /// provider set published with that generation.
    ///
    /// # Parameters
    /// - `stage`: Host operation returning the validated generation candidate,
    ///   complete model rows, and providers bound to that candidate.
    ///
    /// # Returns
    /// The published upstream generation and its retired lease.
    ///
    /// # Errors
    /// Returns an upstream staging or publication error.
    pub fn refresh_provider_extension_generation<E, F>(
        &self,
        stage: F,
    ) -> Result<
        rpi_extensions::PublishedGeneration,
        rpi_model_runtime::ExtensionCatalogRefreshError<E>,
    >
    where
        F: FnOnce() -> Result<
            (
                rpi_extensions::GenerationCandidate,
                Vec<Model>,
                Vec<ProviderAdapter>,
            ),
            E,
        >,
    {
        provider_adapters::publish_provider_extension_generation(
            &self.providers,
            &self.extension_generation,
            &self.model_catalog,
            stage,
        )
    }

    /// Resolves one provider adapter from the currently published provider set.
    ///
    /// # Parameters
    /// - `provider_id`: Exact upstream provider identity.
    ///
    /// # Returns
    /// A clone of the active provider adapter, when registered.
    #[must_use]
    pub fn provider(&self, provider_id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(provider_id)
    }

    /// Captures a session lease and catalog snapshot from one published generation.
    ///
    /// # Returns
    /// The paired upstream extension and provider catalog view.
    #[must_use]
    pub fn extension_catalog_snapshot(&self) -> rpi_model_runtime::ExtensionCatalogSnapshot {
        rpi_model_runtime::snapshot_extension_catalog_generation(
            &self.extension_generation,
            &self.model_catalog,
        )
    }

    /// Captures the paired extension/catalog generation and its active provider identities.
    ///
    /// The provider read lock remains held while the upstream generation
    /// snapshot is captured, preventing refresh from exposing a mixed provider
    /// set and catalog generation.
    ///
    /// # Returns
    /// The upstream extension/catalog snapshot and provider ids published with it.
    #[must_use]
    pub fn provider_generation_snapshot(
        &self,
    ) -> (rpi_model_runtime::ExtensionCatalogSnapshot, Vec<String>) {
        self.providers
            .with_active_ids(|provider_ids| (self.extension_catalog_snapshot(), provider_ids))
    }
}

impl ModelCatalogGeneration {
    /// Names the generation identified by one upstream catalog revision.
    ///
    /// - `revision`: Exact `ModelCatalogSnapshot` revision.
    ///
    /// The corresponding catalog generation.
    /// # Returns
    /// The generation for the supplied revision.
    #[must_use]
    pub const fn from_revision(revision: u64) -> Self {
        Self { revision }
    }

    /// The upstream catalog revision this generation names.
    ///
    /// # Returns
    /// The exact upstream revision value.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
}

/// Reads the current generation published by a shared upstream catalog.
///
/// - `catalog`: Shared live upstream catalog handle.
///
/// The generation identified by that catalog's current revision.
/// # Returns
/// The current catalog generation.
#[must_use]
pub fn catalog_generation(catalog: &ModelCatalog) -> ModelCatalogGeneration {
    ModelCatalogGeneration::from_revision(catalog.snapshot().revision)
}

/// Publishes a complete catalog replacement and returns the new generation.
///
/// The host owns configuration loading, credential and header policy, and the
/// decision to reload; upstream owns atomic publication so readers never
/// observe a mixture of old and new provider rows. Reconciling identical rows
/// is idempotent and leaves the generation unchanged.
///
/// - `catalog`: Shared live upstream catalog handle.
/// - `models`: Complete ordered provider/model rows for the new generation.
///
/// The generation published by this reconciliation.
/// # Returns
/// The generation identifying the committed catalog snapshot.
#[must_use]
pub fn reconcile_model_catalog(
    catalog: &ModelCatalog,
    models: Vec<Model>,
) -> ModelCatalogGeneration {
    ModelCatalogGeneration::from_revision(catalog.reconcile_models_snapshot(models).revision)
}

impl<B> ApplicationService<B> {
    /// Returns the generation currently published by the shared catalog.
    ///
    /// # Returns
    /// The current upstream catalog generation.
    #[must_use]
    pub fn model_catalog_generation(&self) -> ModelCatalogGeneration {
        catalog_generation(&self.model_catalog())
    }

    /// Reconciles the shared live catalog without restarting the process.
    ///
    /// The caller remains the host that resolved the supplied rows; this
    /// operation loads no configuration, resolves no credential, and owns no
    /// package lifecycle. Publication is atomic, so every component sharing
    /// this handle - the model-library projection, selection resolution, plan
    /// assignment, and readiness projection - observes the same new generation.
    ///
    /// - `self`: Service owning the shared catalog handle.
    /// - `models`: Complete ordered provider/model rows for the new generation.
    ///
    /// The generation published by this reload.
    /// # Returns
    /// The new catalog generation.
    #[must_use]
    pub fn reload_model_catalog(&self, models: Vec<Model>) -> ModelCatalogGeneration {
        self.provider_extension_runtime
            .reconcile_model_catalog(models)
    }

    /// Atomically publishes a staged extension candidate and its complete model catalog.
    ///
    /// The host-supplied closure owns package discovery, project-root authorization,
    /// loading, and validation. The application service owns the shared publication
    /// slot and upstream model catalog, so readers observe the replacement as one
    /// extension/catalog generation. A staging error leaves the current pair intact.
    ///
    /// # Parameters
    /// - `self`: Application service owning the shared catalog and generation slot.
    /// - `stage`: Host operation returning a validated extension candidate and all
    ///   model rows for that candidate.
    ///
    /// # Returns
    /// The new extension generation and the retired generation lease.
    ///
    /// # Errors
    /// Returns an upstream `Stage` error when host staging fails, or a `Publish`
    /// error when the upstream publication gate refuses the candidate.
    pub fn refresh_extension_catalog_generation<E, F>(
        &self,
        stage: F,
    ) -> Result<
        rpi_extensions::PublishedGeneration,
        rpi_model_runtime::ExtensionCatalogRefreshError<E>,
    >
    where
        F: FnOnce() -> Result<(rpi_extensions::GenerationCandidate, Vec<Model>), E>,
    {
        self.provider_extension_runtime
            .refresh_extension_catalog_generation(stage)
    }

    /// Captures the extension session and catalog snapshot from one publication generation.
    ///
    /// Readers should use this paired view when selecting extension-owned
    /// providers so a catalog reload cannot be observed with a retired session.
    ///
    /// # Returns
    /// The current extension session lease and matching upstream catalog snapshot.
    #[must_use]
    pub fn extension_catalog_snapshot(&self) -> rpi_model_runtime::ExtensionCatalogSnapshot {
        self.provider_extension_runtime.extension_catalog_snapshot()
    }
}
