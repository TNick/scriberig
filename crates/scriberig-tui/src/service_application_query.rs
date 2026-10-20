//! Implements generic query service adapters for the TUI.

use super::*;

/// Converts the application model rows into protocol compatibility DTOs.
///
/// - `library`: Upstream-derived application model projection.
///
/// Protocol model rows projected from the upstream-derived application rows.
/// # Returns
/// The operation result.
pub(in crate::service) fn model_library_response(
    library: scriberig_app::ModelLibrary,
) -> scriberig_protocol::ModelLibraryResponse {
    scriberig_protocol::ModelLibraryResponse {
        models: library
            .models
            .into_iter()
            .map(|model| scriberig_protocol::PublicModelDescriptor {
                id: model.id.as_str().to_owned(),
                display_name: model.metadata_snapshot.display_name,
                provider_id: model.metadata_snapshot.provider_id,
                provider_model_id: model.metadata_snapshot.model_id,
                thinking_levels: model
                    .metadata_snapshot
                    .thinking_levels
                    .into_iter()
                    .map(|level| level.as_str().to_owned())
                    .collect(),
                metadata_snapshot: scriberig_protocol::PublicModelMetadataSnapshot {
                    schema_version: model.metadata_snapshot.schema_version,
                    api_id: model.metadata_snapshot.api_id,
                    reasoning: model.metadata_snapshot.reasoning,
                    input: model
                        .metadata_snapshot
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
                        input: model.metadata_snapshot.cost.rates.input,
                        output: model.metadata_snapshot.cost.rates.output,
                        cache_read: model.metadata_snapshot.cost.rates.cache_read,
                        cache_write: model.metadata_snapshot.cost.rates.cache_write,
                        tiers: model
                            .metadata_snapshot
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
                    cost_configured: model.metadata_snapshot.cost_configured,
                    context_window: model.metadata_snapshot.context_window,
                    max_tokens: model.metadata_snapshot.max_tokens,
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
