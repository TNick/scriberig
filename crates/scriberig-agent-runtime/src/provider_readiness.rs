//! Projects host-authorized credential readiness for one catalog generation.
//!
//! Upstream `rpi-model-runtime` owns the credential-source vocabulary and
//! catalog-header detection, and `scriberig-secrets` owns destination
//! authorization. This module joins them for one coherent catalog snapshot so
//! the safe status rows a surface displays are bound to the catalog generation
//! those rows were resolved from. The result is credential availability only:
//! it never claims that a provider accepted a credential, and no credential
//! material, endpoint, or header value leaves this module.
//!
//! `ProviderReadinessProjection::aligned_rows_for` is the single rule surfaces
//! use to decide whether these rows may be paired with a model-library response,
//! so no transport or terminal adapter re-implements the generation comparison.

use std::collections::BTreeSet;

use rpi_model_runtime::{CredentialSource as UpstreamCredentialSource, ModelCatalog};
use scriberig_app::ModelCatalogGeneration;
use scriberig_protocol::{ModelLibraryResponse, ProviderCredentialReadiness, ProviderReadiness};
use scriberig_secrets::CredentialProvider;

use crate::protected_credential::credential_source_for_model;

#[cfg(test)]
#[path = "provider_readiness/tests.rs"]
mod tests;

/// Safe credential readiness rows bound to one upstream catalog generation.
///
/// # Public members
/// - `generation`: Catalog generation the rows were resolved from.
/// - `rows`: Per-model readiness rows containing no credential material.
///
/// # Private members
/// None.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderReadinessProjection {
    /// Upstream catalog generation these rows were resolved from.
    pub generation: ModelCatalogGeneration,
    /// Credential-safe readiness rows for that generation.
    pub rows: Vec<ProviderReadiness>,
}

impl ProviderReadinessProjection {
    /// Reports whether these rows were resolved from `generation`.
    ///
    /// - `generation`: Generation of the catalog rows shown alongside these
    ///   readiness rows.
    ///
    /// `true` when a reload has not superseded the observed rows.
    /// # Returns
    /// Whether the two views came from the same catalog generation.
    #[must_use]
    pub fn is_aligned_to(&self, generation: &ModelCatalogGeneration) -> bool {
        &self.generation == generation
    }

    /// Returns the rows a surface may pair with one model-library response.
    ///
    /// A host catalog reload can replace a model's endpoint between the
    /// model-library read and this projection, so a caller supplies the
    /// generation its model rows were read from. Upstream catalog revisions only
    /// advance, so an aligned projection proves no reload superseded those rows.
    /// Rows naming a provider/model pair absent from the response are dropped.
    ///
    /// - `response`: Model-library response being displayed.
    /// - `generation`: Catalog generation those model rows came from.
    ///
    /// The credential-safe rows for that generation, filtered to the response's
    /// provider/model pairs.
    /// # Returns
    /// `None` when a reload superseded the displayed model rows.
    #[must_use]
    pub fn aligned_rows_for(
        &self,
        response: &ModelLibraryResponse,
        generation: &ModelCatalogGeneration,
    ) -> Option<Vec<ProviderReadiness>> {
        if !self.is_aligned_to(generation) {
            return None;
        }
        let displayed = response
            .models
            .iter()
            .map(|model| (model.provider_id.as_str(), model.provider_model_id.as_str()))
            .collect::<BTreeSet<_>>();
        Some(
            self.rows
                .iter()
                .filter(|row| {
                    displayed.contains(&(row.provider_id.as_str(), row.model_id.as_str()))
                })
                .cloned()
                .collect(),
        )
    }
}

/// Projects credential readiness for every model in one catalog snapshot.
///
/// The catalog is snapshotted once, so all rows and the returned generation
/// come from the same catalog state. A host-authorized credential is resolved
/// per model against that model's exact endpoint; a host authorization or
/// resolution failure is reported as `Denied` rather than degraded to a missing
/// credential, and the resolved material is dropped before projection.
///
/// - `catalog`: Shared live upstream model catalog.
/// - `credentials`: Host credential authority, or `None` when the host path has
///   no authorized credential source for these providers.
///
/// The generation-aligned, credential-safe readiness projection.
/// # Returns
/// One readiness row per distinct provider/model pair in the snapshot, in
/// catalog order.
#[must_use]
pub fn project_readiness(
    catalog: &ModelCatalog,
    credentials: Option<&dyn CredentialProvider>,
) -> ProviderReadinessProjection {
    let snapshot = catalog.snapshot();
    let mut seen = std::collections::BTreeSet::new();
    let rows = snapshot
        .models
        .iter()
        .filter(|model| seen.insert((model.provider.clone(), model.id.clone())))
        .map(|model| ProviderReadiness {
            provider_id: model.provider.clone(),
            model_id: model.id.clone(),
            credential: classify_credential(model, credentials),
        })
        .collect();
    ProviderReadinessProjection {
        generation: ModelCatalogGeneration::from_revision(snapshot.revision),
        rows,
    }
}

/// Classifies one model's credential readiness without exposing material.
///
/// - `model`: Exact upstream model whose endpoint is authorized.
/// - `credentials`: Host credential authority, when this path has one.
///
/// The safe readiness category. A host authorization failure is `Denied`, an
/// unresolvable credential is `Missing`, and upstream catalog header auth stays
/// distinguishable from a host credential.
/// # Returns
/// The credential readiness category for this model.
fn classify_credential(
    model: &rpi_ai::Model,
    credentials: Option<&dyn CredentialProvider>,
) -> ProviderCredentialReadiness {
    let source = match credentials {
        Some(credentials) => credential_source_for_model(credentials, model).map_err(|_| ()),
        None => Ok(rpi_model_runtime::readiness_from_catalog(
            &model.provider,
            UpstreamCredentialSource::Missing,
            std::slice::from_ref(model),
        )),
    };
    match source {
        Err(()) => ProviderCredentialReadiness::Denied,
        Ok(
            UpstreamCredentialSource::HostSelected
            | UpstreamCredentialSource::CliOverride
            | UpstreamCredentialSource::StoredApiKey
            | UpstreamCredentialSource::StoredOauth
            | UpstreamCredentialSource::ModelsJson
            | UpstreamCredentialSource::Environment(_),
        ) => ProviderCredentialReadiness::Available,
        Ok(UpstreamCredentialSource::Headers) => ProviderCredentialReadiness::CatalogHeader,
        Ok(UpstreamCredentialSource::Anonymous) => ProviderCredentialReadiness::NotRequired,
        Ok(UpstreamCredentialSource::Missing) => ProviderCredentialReadiness::Missing,
    }
}
