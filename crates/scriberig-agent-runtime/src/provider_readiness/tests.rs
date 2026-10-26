//! Verifies credential-safe readiness stays bound to one catalog generation
//! and is only offered to surfaces displaying the same generation.

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

/// Verifies a host may resolve credential readiness independently per model.
#[test]
fn per_model_credential_sources_remain_bound_to_their_catalog_rows() {
    let catalog = catalog_with(vec![
        model("provider-a", "model-a"),
        model("provider-a", "model-b"),
    ]);

    let projection = project_readiness_with(&catalog, |model| {
        Ok(if model.id == "model-a" {
            UpstreamCredentialSource::HostSelected
        } else {
            UpstreamCredentialSource::Missing
        })
    });

    assert_eq!(projection.rows[0].model_id, "model-a");
    assert_eq!(
        projection.rows[0].credential,
        ProviderCredentialReadiness::Available
    );
    assert_eq!(projection.rows[1].model_id, "model-b");
    assert_eq!(
        projection.rows[1].credential,
        ProviderCredentialReadiness::Missing
    );
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

/// Builds a model-library response listing the supplied provider/model rows.
///
/// - `pairs`: Provider and provider-native model identities to display.
///
/// The response a surface pairs readiness rows with.
/// # Returns
/// A model-library response containing one row per pair.
fn library(pairs: &[(&str, &str)]) -> scriberig_protocol::ModelLibraryResponse {
    scriberig_protocol::ModelLibraryResponse {
        models: pairs
            .iter()
            .map(
                |(provider_id, model_id)| scriberig_protocol::PublicModelDescriptor {
                    id: scriberig_protocol::model_row_id(provider_id, model_id),
                    display_name: (*model_id).to_owned(),
                    provider_id: (*provider_id).to_owned(),
                    provider_model_id: (*model_id).to_owned(),
                    thinking_levels: Vec::new(),
                    metadata_snapshot: scriberig_protocol::PublicModelMetadataSnapshot {
                        schema_version: "rpi-safe-model-snapshot-v1".to_owned(),
                        api_id: "openai-completions".to_owned(),
                        reasoning: false,
                        input: Vec::new(),
                        cost: scriberig_protocol::PublicModelCostSnapshot {
                            input: 0.0,
                            output: 0.0,
                            cache_read: 0.0,
                            cache_write: 0.0,
                            tiers: Vec::new(),
                        },
                        cost_configured: false,
                        context_window: 0,
                        max_tokens: 0,
                    },
                },
            )
            .collect(),
        provider_readiness: Vec::new(),
    }
}

/// Builds a readiness projection carrying one status row per pair.
///
/// - `revision`: Catalog generation the rows were resolved from.
/// - `pairs`: Provider and provider-native model identities.
///
/// The projection under test.
/// # Returns
/// A projection with one `Available` row per pair.
fn projection(revision: u64, pairs: &[(&str, &str)]) -> ProviderReadinessProjection {
    ProviderReadinessProjection {
        generation: scriberig_app::ModelCatalogGeneration::from_revision(revision),
        rows: pairs
            .iter()
            .map(|(provider_id, model_id)| ProviderReadiness {
                provider_id: (*provider_id).to_owned(),
                model_id: (*model_id).to_owned(),
                credential: ProviderCredentialReadiness::Available,
                authentication: None,
            })
            .collect(),
    }
}

/// Verifies aligned rows are kept and rows for undisplayed models are dropped.
#[test]
fn aligned_rows_are_filtered_to_the_displayed_models() {
    let observed = scriberig_app::ModelCatalogGeneration::from_revision(4);
    let resolved = projection(4, &[("provider-a", "model-a"), ("provider-b", "model-b")]);

    let rows = resolved
        .aligned_rows_for(&library(&[("provider-a", "model-a")]), &observed)
        .expect("aligned generation");

    assert_eq!(
        rows.iter()
            .map(|row| (row.provider_id.as_str(), row.model_id.as_str()))
            .collect::<Vec<_>>(),
        vec![("provider-a", "model-a")]
    );
}

/// Verifies a projection from a superseded generation offers no rows.
#[test]
fn superseded_projection_offers_no_rows() {
    let observed = scriberig_app::ModelCatalogGeneration::from_revision(4);
    let resolved = projection(5, &[("provider-a", "model-a")]);

    assert!(
        resolved
            .aligned_rows_for(&library(&[("provider-a", "model-a")]), &observed)
            .is_none()
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
