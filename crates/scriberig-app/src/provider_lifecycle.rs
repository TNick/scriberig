//! Defines the shared provider/model catalog generation and reload contract.
//!
//! Upstream `rpi-model-runtime` owns the live model catalog, its revision
//! counter, and atomic reconciliation. This module only names the generation a
//! consumer observed and publishes a complete host-supplied replacement through
//! the shared catalog, so model rows and credential readiness can be
//! reconciled without restarting the process. Provider package discovery,
//! installation, and reload remain upstream lifecycle gates; `ScribeRig` does not
//! own a package registry or a second catalog.

use rpi_ai::Model;
use rpi_model_runtime::ModelCatalog;

use crate::ApplicationService;

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
    pub fn reload_model_catalog(&self, models: Vec<Model>) -> ModelCatalogGeneration {
        reconcile_model_catalog(&self.model_catalog(), models)
    }
}
