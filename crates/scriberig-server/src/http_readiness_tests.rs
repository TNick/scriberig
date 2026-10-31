//! Verifies the HTTP transport pairs credential readiness with model rows from
//! one upstream catalog generation.

use super::attach_provider_readiness;
use crate::http::{HttpState, http_protocol::dispatch};
use crate::server::ApplicationService;
use scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection;
use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, ModelCatalogGeneration,
    RuntimeCapability,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{ModelLibraryResponse, ProviderCredentialReadiness, ProviderReadiness};

/// Provider/model pair displayed by the fixture service.
const DISPLAYED: (&str, &str) = ("provider-a", "model-a");

/// A provider/model pair the fixture service does not display.
const UNDISPLAYED: (&str, &str) = ("provider-b", "model-b");

/// Application service reporting model rows plus an optional catalog generation.
///
/// # Public members
/// - `generation`: Generation the fixture's model rows come from, or `None`
///   when this service has no upstream model catalog.
///
/// # Private members
/// None.
struct ReadinessFixture {
    /// Upstream catalog revision the fixture's model rows were read from.
    generation: Option<u64>,
}

impl ApplicationService for ReadinessFixture {
    /// Rejects inspection because this fixture only serves model rows.
    ///
    /// - `_context`: Request authorization context.
    /// - `_request`: Inspection request, intentionally unsupported.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns unsupported-operation for every inspection request.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        _request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "inspect".to_owned(),
        })
    }

    /// Reports no runtime capabilities for this fixture.
    ///
    /// An empty capability list.
    /// # Returns
    /// The runtime capabilities this service advertises.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        Vec::new()
    }

    /// Rejects operation status because this fixture has no operations.
    ///
    /// - `_operation_id`: Operation identifier, intentionally unsupported.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns unsupported-operation for every operation status read.
    fn operation_status(
        &self,
        _operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation status".to_owned(),
        })
    }

    /// Returns one credential-safe model row.
    ///
    /// - `_context`: Trusted caller context.
    ///
    /// The fixture model-library response, which carries no readiness rows of
    /// its own.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Never fails for this fixture.
    fn model_library(
        &self,
        _context: &ApplicationContext,
    ) -> Result<ModelLibraryResponse, ApplicationError> {
        Ok(library_response(&[DISPLAYED]))
    }

    /// Reports the generation the fixture's model rows were read from.
    ///
    /// # Returns
    /// The configured generation, or `None` when the fixture has no upstream
    /// model catalog.
    fn model_catalog_generation(&self) -> Option<ModelCatalogGeneration> {
        self.generation.map(ModelCatalogGeneration::from_revision)
    }
}

/// Builds a model-library response listing the supplied provider/model rows.
///
/// - `pairs`: Exact provider and provider-native model identities to display.
///
/// A response with one display row per pair and no readiness rows.
/// # Returns
/// The model-library response under test.
fn library_response(pairs: &[(&str, &str)]) -> ModelLibraryResponse {
    ModelLibraryResponse {
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

/// Builds a host readiness projection resolved from one catalog generation.
///
/// - `revision`: Upstream catalog revision the rows were resolved from.
/// - `pairs`: Provider/model identities the host reported status for.
///
/// A credential-safe projection containing one `Available` row per pair.
/// # Returns
/// The readiness projection under test.
fn projection(revision: u64, pairs: &[(&str, &str)]) -> ProviderReadinessProjection {
    ProviderReadinessProjection {
        generation: ModelCatalogGeneration::from_revision(revision),
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

/// Builds one HTTP state over the readiness fixture.
///
/// - `generation`: Generation reported by the fixture service.
/// - `handler`: Host readiness handler, when the composition has one.
///
/// The state passed to the operation-segment dispatcher.
/// # Returns
/// A transport state for one model-library read.
fn state(
    generation: Option<u64>,
    handler: Option<std::sync::Arc<crate::ProviderReadinessHandler>>,
) -> HttpState<ReadinessFixture> {
    HttpState {
        service: ReadinessFixture { generation },
        principal: scriberig_app::Principal::new("readiness-test"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: None,
        provider_readiness_handler: handler,
        fixture: crate::development::DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    }
}

/// Returns the `provider_readiness` array of a dispatched model-library response.
///
/// - `generation`: Generation reported by the fixture service.
/// - `resolved_revision`: Generation the host readiness rows were resolved from.
///
/// The serialized readiness rows the transport attached, if any.
/// # Returns
/// The attached rows, or `None` when the response omitted them.
fn dispatched_readiness(generation: Option<u64>, resolved_revision: u64) -> Option<Vec<String>> {
    let handler: std::sync::Arc<crate::ProviderReadinessHandler> =
        std::sync::Arc::new(move || Some(projection(resolved_revision, &[DISPLAYED, UNDISPLAYED])));
    let response = dispatch(
        &state(generation, Some(handler)),
        "model_library",
        "{}",
        "model-library-1".to_owned(),
    )
    .expect("model library response");
    response["response"]["ModelLibrary"]
        .get("provider_readiness")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| row["model_id"].as_str().unwrap_or_default().to_owned())
                .collect()
        })
}

/// Verifies aligned readiness rows attach and undisplayed models are dropped.
#[test]
fn aligned_readiness_is_attached_and_filtered() {
    let mut response = library_response(&[DISPLAYED]);

    attach_provider_readiness(
        &mut response,
        Some(projection(4, &[DISPLAYED, UNDISPLAYED])),
        Some(ModelCatalogGeneration::from_revision(4)),
    );

    assert_eq!(
        response
            .provider_readiness
            .iter()
            .map(|row| (row.provider_id.as_str(), row.model_id.as_str()))
            .collect::<Vec<_>>(),
        vec![DISPLAYED]
    );
}

/// Verifies rows resolved from a superseded generation are not attached.
#[test]
fn superseded_readiness_is_omitted() {
    let mut response = library_response(&[DISPLAYED]);

    attach_provider_readiness(
        &mut response,
        Some(projection(5, &[DISPLAYED])),
        Some(ModelCatalogGeneration::from_revision(4)),
    );

    assert_eq!(response.provider_readiness, Vec::new());
}

/// Verifies a service without an upstream catalog cannot prove alignment.
#[test]
fn missing_service_generation_omits_readiness() {
    let mut response = library_response(&[DISPLAYED]);

    attach_provider_readiness(&mut response, Some(projection(4, &[DISPLAYED])), None);

    assert_eq!(response.provider_readiness, Vec::new());
}

/// Verifies a host without a readiness source leaves the field empty.
#[test]
fn absent_host_projection_keeps_readiness_empty() {
    let mut response = library_response(&[DISPLAYED]);

    attach_provider_readiness(
        &mut response,
        None,
        Some(ModelCatalogGeneration::from_revision(4)),
    );

    assert_eq!(response.provider_readiness, Vec::new());
}

/// Verifies the model-library route attaches readiness resolved from the same
/// generation the model rows came from.
#[test]
fn model_library_route_attaches_generation_aligned_readiness() {
    let rows = dispatched_readiness(Some(7), 7).expect("aligned readiness rows");

    assert_eq!(rows, vec!["model-a".to_owned()]);
}

/// Verifies the model-library route omits readiness a reload superseded.
#[test]
fn model_library_route_omits_superseded_readiness() {
    assert_eq!(dispatched_readiness(Some(7), 8), None);
}

/// Verifies the model-library route omits readiness when the service cannot
/// name the generation its model rows came from.
#[test]
fn model_library_route_omits_readiness_without_a_catalog_generation() {
    assert_eq!(dispatched_readiness(None, 7), None);
}
