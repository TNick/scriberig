//! Implements generic query service adapters for the TUI.

use super::*;

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

impl<S: QueryService> TuiService for S {
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
