//! Implements generic query service adapters for the TUI.

use super::*;
use super::{TuiRecoveryService, TuiRuntimeService};

/// Converts upstream safe model snapshots into protocol DTOs.
///
/// - `models`: Upstream-owned display-safe model snapshots.
///
/// Protocol model rows projected at the client boundary.
/// # Returns
/// The operation result.
pub(in crate::service) fn model_library_response(
    models: Vec<rpi_model_runtime::SafeModelSnapshot>,
) -> scriberig_protocol::ModelLibraryResponse {
    scriberig_protocol::ModelLibraryResponse {
        provider_readiness: Vec::new(),
        models: models
            .into_iter()
            .map(|model| scriberig_protocol::PublicModelDescriptor {
                id: scriberig_protocol::model_row_id(&model.provider_id, &model.model_id),
                display_name: model.display_name,
                provider_id: model.provider_id,
                provider_model_id: model.model_id,
                thinking_levels: model
                    .thinking_levels
                    .into_iter()
                    .map(|level| level.as_str().to_owned())
                    .collect(),
                metadata_snapshot: scriberig_protocol::PublicModelMetadataSnapshot {
                    schema_version: model.schema_version,
                    api_id: model.api_id,
                    reasoning: model.reasoning,
                    input: model
                        .input
                        .into_iter()
                        .map(|modality| match modality {
                            rpi_ai::InputModality::Text => {
                                scriberig_protocol::PublicModelInputModality::Text
                            }
                            rpi_ai::InputModality::Image => {
                                scriberig_protocol::PublicModelInputModality::Image
                            }
                        })
                        .collect(),
                    cost: scriberig_protocol::PublicModelCostSnapshot {
                        input: model.cost.rates.input,
                        output: model.cost.rates.output,
                        cache_read: model.cost.rates.cache_read,
                        cache_write: model.cost.rates.cache_write,
                        tiers: model
                            .cost
                            .tiers
                            .into_iter()
                            .map(|tier| scriberig_protocol::PublicModelCostTierSnapshot {
                                input: tier.rates.input,
                                output: tier.rates.output,
                                cache_read: tier.rates.cache_read,
                                cache_write: tier.rates.cache_write,
                                input_tokens_above: tier.input_tokens_above,
                            })
                            .collect(),
                    },
                    cost_configured: model.cost_configured,
                    context_window: model.context_window,
                    max_tokens: model.max_tokens,
                },
            })
            .collect(),
    }
}

/// Attaches host credential readiness only when it matches the model rows.
///
/// A completed catalog reload can replace a model's endpoint between the
/// model-library read and the host readiness projection. Readiness rows
/// resolved from a different upstream catalog generation are omitted instead of
/// pairing a credential status with a superseded model row, and rows for models
/// the response does not display are dropped.
///
/// - `response`: Model-library response to update in place.
/// - `readiness`: Host readiness projection, when this runtime has one.
/// - `generation`: Catalog generation the response's model rows came from.
pub(in crate::service) fn attach_provider_readiness(
    response: &mut scriberig_protocol::ModelLibraryResponse,
    readiness: Option<scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection>,
    generation: scriberig_app::ModelCatalogGeneration,
) {
    if let Some(rows) = readiness.and_then(|rows| rows.aligned_rows_for(response, &generation)) {
        response.provider_readiness = rows;
    }
}

impl<S: QueryService> TuiService for S {
    /// Runs a geospatial query through the shared application query service.
    ///
    /// # Parameters
    /// - `context`: Caller capability context.
    /// - `request`: Exact query scope, predicate, and result bounds.
    ///
    /// # Returns
    /// Typed matches and query provenance normalized for terminal display.
    ///
    /// # Errors
    /// Returns the shared service's authorization or query error.
    fn query_geospatial_features(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::GeoFeatureQuery,
    ) -> Result<scriberig_protocol::GeoFeatureQueryResponse, ApplicationError> {
        QueryService::query_geospatial_features(self, context, request)
            .map(crate::view::geo::query_from_application)
    }

    /// Reads a QGIS project through the shared exact-snapshot query contract.
    ///
    /// # Parameters
    /// - `context`: Caller capability context.
    /// - `request`: Exact project, path, and snapshot scope.
    ///
    /// # Returns
    /// Provider-neutral QGIS structure with linked semantic layer identities.
    ///
    /// # Errors
    /// Returns the shared service's authorization or source error.
    fn inspect_qgis_project(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::QgisProjectQuery,
    ) -> Result<scriberig_protocol::QgisProjectResponse, ApplicationError> {
        let result = QueryService::inspect_qgis_project(self, context, request)?;
        let model = result.qgis.project;
        Ok(scriberig_protocol::QgisProjectResponse {
            project: result.owner.to_string(),
            path: request.path.clone(),
            snapshot: result.qgis.snapshot.to_string(),
            revision: result.qgis.revision.to_string(),
            id: model.id.to_string(),
            title: model.title,
            crs: model.crs.map(|crs| scriberig_protocol::GeoCrsView {
                authority: crs.authority,
                code: crs.code,
                definition: crs.definition,
            }),
            layers: model
                .layers
                .into_iter()
                .map(|layer| scriberig_protocol::QgisLayerReferenceView {
                    id: layer.id.to_string(),
                    name: layer.name,
                    data_source: layer.source.raw,
                    path: layer.source.path,
                    resolved_path: layer.resolved_path,
                    source_layer: layer.source_layer,
                    dataset: layer.dataset.map(|id| id.to_string()),
                    layer: layer.layer.map(|id| id.to_string()),
                    resolution_diagnostic: layer.resolution_diagnostic,
                    visible: layer.presentation.visible,
                    renderer: layer.presentation.renderer,
                    scale_range: layer.presentation.scale_range,
                })
                .collect(),
            groups: model
                .groups
                .into_iter()
                .map(|group| scriberig_protocol::QgisLayerGroupView {
                    id: group.id.to_string(),
                    name: group.name,
                    layers: group.layers.into_iter().map(|id| id.to_string()).collect(),
                })
                .collect(),
        })
    }

    /// Inspects one exact coordinate through the shared application query contract.
    ///
    /// # Parameters
    /// - `self`: Shared query service.
    /// - `context`: Caller authorization context.
    /// - `request`: Exact project, snapshot, worksheet, row, and column.
    ///
    /// # Returns
    /// Typed cell details normalized to the protocol response shape.
    ///
    /// # Errors
    /// Returns the query service's structured authorization, request, or storage failure.
    fn inspect_spreadsheet_cell(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::SpreadsheetCellQuery,
    ) -> Result<scriberig_protocol::SpreadsheetCellInspectResponse, ApplicationError> {
        QueryService::inspect_spreadsheet_cell(self, context, request)
            .map(crate::view::spreadsheet::cell_inspection_from_application)
    }

    /// Reads a bounded exact-snapshot spreadsheet page through the query service.
    ///
    /// # Parameters
    /// - `self`: Shared query service.
    /// - `context`: Caller authorization context.
    /// - `request`: Exact project, snapshot, worksheet, range, and page bounds.
    ///
    /// # Returns
    /// Typed cells and continuation metadata normalized to the protocol shape.
    ///
    /// # Errors
    /// Returns the query service's structured authorization, request, or storage failure.
    fn inspect_spreadsheet_cells(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::SpreadsheetCellsQuery,
    ) -> Result<scriberig_protocol::SpreadsheetCellsResponse, ApplicationError> {
        self.read_spreadsheet_cells(context, request)
            .map(crate::view::spreadsheet::from_application)
    }

    /// Delegates inspection through a public query service.
    ///
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// The inspection result.
    ///
    /// Returns the query service error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }
}

impl<S: QueryService> TuiModelRequestService for S {}

impl<S: QueryService> TuiRecoveryService for S {}

impl<S: QueryService> TuiRuntimeService for S {}

#[cfg(test)]
mod tests {
    //! Verifies readiness rows stay bound to the displayed model generation.

    use super::*;

    /// Builds a one-row model-library response.
    ///
    /// # Returns
    /// A response containing one provider/model display row.
    fn response() -> scriberig_protocol::ModelLibraryResponse {
        scriberig_protocol::ModelLibraryResponse {
            models: vec![scriberig_protocol::PublicModelDescriptor {
                id: scriberig_protocol::model_row_id("provider-a", "model-a"),
                display_name: "Model A".to_owned(),
                provider_id: "provider-a".to_owned(),
                provider_model_id: "model-a".to_owned(),
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
            }],
            provider_readiness: Vec::new(),
        }
    }

    /// Builds a readiness projection carrying one status row.
    ///
    /// - `revision`: Catalog generation the rows were resolved from.
    ///
    /// # Returns
    /// The projection used by the attachment rule.
    fn readiness(
        revision: u64,
    ) -> scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection {
        scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection {
            generation: scriberig_app::ModelCatalogGeneration::from_revision(revision),
            rows: vec![scriberig_protocol::ProviderReadiness {
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
                credential: scriberig_protocol::ProviderCredentialReadiness::Available,
                authentication: None,
            }],
        }
    }

    /// Verifies aligned readiness rows are attached to the model rows.
    #[test]
    fn aligned_readiness_is_attached() {
        let mut response = response();

        attach_provider_readiness(
            &mut response,
            Some(readiness(3)),
            scriberig_app::ModelCatalogGeneration::from_revision(3),
        );

        assert_eq!(response.provider_readiness, readiness(3).rows);
    }

    /// Verifies rows from a superseded generation are not attached.
    #[test]
    fn superseded_readiness_is_omitted() {
        let mut response = response();

        attach_provider_readiness(
            &mut response,
            Some(readiness(2)),
            scriberig_app::ModelCatalogGeneration::from_revision(3),
        );

        assert_eq!(response.provider_readiness, Vec::new());
    }

    /// Verifies a runtime without a host readiness source leaves the field empty.
    #[test]
    fn absent_host_readiness_stays_empty() {
        let mut response = response();

        attach_provider_readiness(
            &mut response,
            None,
            scriberig_app::ModelCatalogGeneration::from_revision(0),
        );

        assert_eq!(response.provider_readiness, Vec::new());
    }
}
