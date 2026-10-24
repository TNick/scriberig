//! Projects host-authorized credential readiness for one catalog generation.
//!
//! Upstream `rpi-model-runtime` owns the credential-source vocabulary and
//! catalog-header detection, and `scriberig-secrets` owns destination
//! authorization. This module joins them for one coherent catalog snapshot so
//! the safe status rows a surface displays are bound to the catalog generation
//! those rows were resolved from. The result is credential availability only:
//! it never claims that a provider accepted a credential, and no credential
//! material, endpoint, or header value leaves this module.

use rpi_model_runtime::{CredentialSource as UpstreamCredentialSource, ModelCatalog};
use scriberig_app::ModelCatalogGeneration;
use scriberig_protocol::{ProviderCredentialReadiness, ProviderReadiness};
use scriberig_secrets::CredentialProvider;

use crate::protected_credential::credential_source_for_model;

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

#[cfg(test)]
mod tests {
    //! Verifies credential-safe readiness stays bound to one catalog generation.

    use super::*;
    use rpi_ai::{Api, Model};
    use scriberig_secrets::{SecretError, SecretValue};

    /// Credential sentinel that must never reach a readiness row.
    const SENTINEL: &str = "sentinel-credential-material";

    /// Fixed host credential behaviour for readiness fixtures.
    enum FixtureCredentials {
        /// The host authority denies or fails to resolve the reference.
        Denied,
        /// No credential is configured for this destination.
        Absent,
        /// An authorized credential is available.
        Present,
    }

    /// Host credential stub returning one fixed outcome.
    struct StubCredentialProvider {
        /// Fixed outcome for every authorized destination.
        outcome: FixtureCredentials,
    }

    impl CredentialProvider for StubCredentialProvider {
        /// Returns the fixed outcome without inspecting the endpoint value.
        ///
        /// # Parameters
        /// - `_endpoint`: Exact provider endpoint for the request.
        ///
        /// # Returns
        /// The fixture credential outcome.
        ///
        /// # Errors
        /// Returns a denial when the fixture models host authorization failure.
        fn credential(&self, _endpoint: &str) -> Result<Option<SecretValue>, SecretError> {
            match self.outcome {
                FixtureCredentials::Denied => Err(SecretError::Denied),
                FixtureCredentials::Absent => Ok(None),
                FixtureCredentials::Present => Ok(Some(
                    SecretValue::new(SENTINEL.to_owned()).expect("valid secret"),
                )),
            }
        }
    }

    /// Builds one credential-free upstream model row.
    ///
    /// - `provider_id`: Upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    ///
    /// The upstream model row for catalog fixtures.
    /// # Returns
    /// The credential-free upstream model row.
    fn model(provider_id: &str, model_id: &str) -> Model {
        let mut model = Model::new(
            model_id,
            model_id,
            Api::OpenaiCompletions,
            provider_id,
            "https://provider.example.test/v1",
        );
        model.headers = None;
        model
    }

    /// Builds a two-provider catalog.
    ///
    /// - `models`: Complete rows to publish.
    ///
    /// The shared live catalog for a readiness fixture.
    /// # Returns
    /// The catalog publishing the supplied rows.
    fn catalog_with(models: Vec<Model>) -> ModelCatalog {
        let catalog = ModelCatalog::new();
        catalog.reconcile_models(models);
        catalog
    }

    /// Verifies a host denial is reported as denied, never as a missing source.
    #[test]
    fn authorization_denial_is_not_degraded_to_missing() {
        let catalog = catalog_with(vec![model("provider-a", "model-a")]);
        let denied = StubCredentialProvider {
            outcome: FixtureCredentials::Denied,
        };

        let projection = project_readiness(&catalog, Some(&denied));

        assert_eq!(projection.rows.len(), 1);
        assert_eq!(
            projection.rows[0].credential,
            ProviderCredentialReadiness::Denied
        );
        assert_eq!(projection.rows[0].provider_id, "provider-a");
        assert_eq!(projection.rows[0].model_id, "model-a");
    }

    /// Verifies an unconfigured credential reports a safe missing source.
    #[test]
    fn absent_credential_reports_missing() {
        let catalog = catalog_with(vec![model("provider-a", "model-a")]);
        let absent = StubCredentialProvider {
            outcome: FixtureCredentials::Absent,
        };

        let projection = project_readiness(&catalog, Some(&absent));

        assert_eq!(
            projection.rows[0].credential,
            ProviderCredentialReadiness::Missing
        );
    }

    /// Verifies an authorized credential reports availability without material.
    #[test]
    fn available_credential_projects_no_material() {
        let catalog = catalog_with(vec![model("provider-a", "model-a")]);
        let present = StubCredentialProvider {
            outcome: FixtureCredentials::Present,
        };

        let projection = project_readiness(&catalog, Some(&present));

        assert_eq!(
            projection.rows[0].credential,
            ProviderCredentialReadiness::Available
        );
        let serialized = serde_json::to_string(&projection.rows).expect("serializable rows");
        assert!(!serialized.contains(SENTINEL));
        assert!(!serialized.contains("provider.example.test"));
    }

    /// Verifies a host with no credential path still reports header auth.
    #[test]
    fn catalog_header_auth_stays_distinguishable_without_a_host_credential() {
        let mut header_model = model("provider-a", "model-a");
        header_model.headers = Some(std::collections::BTreeMap::from([(
            "authorization".to_owned(),
            SENTINEL.to_owned(),
        )]));
        let catalog = catalog_with(vec![header_model]);

        let projection = project_readiness(&catalog, None);

        assert_eq!(
            projection.rows[0].credential,
            ProviderCredentialReadiness::CatalogHeader
        );
        let serialized = serde_json::to_string(&projection.rows).expect("serializable rows");
        assert!(!serialized.contains(SENTINEL));
    }

    /// Verifies readiness rows carry the generation of their own snapshot.
    #[test]
    fn readiness_rows_are_bound_to_one_catalog_generation() {
        let catalog = catalog_with(vec![model("provider-a", "model-a")]);
        let observed = project_readiness(&catalog, None);
        let current = scriberig_app::catalog_generation(&catalog);

        assert!(observed.is_aligned_to(&current));

        let reloaded = scriberig_app::reconcile_model_catalog(
            &catalog,
            vec![
                model("provider-b", "model-b"),
                model("provider-a", "model-a"),
            ],
        );

        assert!(reloaded.revision() > observed.generation.revision());
        assert!(!observed.is_aligned_to(&reloaded));
        assert_eq!(reloaded, scriberig_app::catalog_generation(&catalog));
        let refreshed = project_readiness(&catalog, None);
        assert!(refreshed.is_aligned_to(&reloaded));
        assert_eq!(
            refreshed
                .rows
                .iter()
                .map(|row| (row.provider_id.as_str(), row.model_id.as_str()))
                .collect::<Vec<_>>(),
            vec![("provider-b", "model-b"), ("provider-a", "model-a")]
        );
    }

    /// Verifies an idempotent reload does not advance the generation.
    #[test]
    fn unchanged_reload_keeps_the_generation() {
        let models = vec![model("provider-a", "model-a")];
        let catalog = catalog_with(models.clone());
        let before = scriberig_app::catalog_generation(&catalog);

        let after = scriberig_app::reconcile_model_catalog(&catalog, models);

        assert_eq!(after, before);
        assert_eq!(after, scriberig_app::catalog_generation(&catalog));
    }
}
