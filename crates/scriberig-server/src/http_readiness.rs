//! Pairs host credential readiness with model rows from one catalog generation.
//!
//! Upstream `rpi-model-runtime` owns the live model catalog and its revision,
//! and the host owns the authorized credential projection. This module only
//! decides whether a readiness projection may be attached to a model-library
//! response that this transport just assembled. It adds no readiness
//! classification, no catalog, and no status cache.

use scriberig_protocol::ModelLibraryResponse;

#[cfg(test)]
#[path = "http_readiness_tests.rs"]
mod tests;

/// Attaches host credential readiness only while it matches the model rows.
///
/// A host catalog reload replaces the model rows and advances the upstream
/// catalog revision, so the transport reads the generation before dispatching
/// the model-library read: a projection aligned to that generation proves no
/// reload superseded the rows in this response. A projection from any other
/// generation, and a host with no upstream catalog generation to compare
/// against, leave the response's readiness rows empty instead of pairing
/// credential status with superseded model rows.
///
/// - `response`: Model-library response to update in place.
/// - `readiness`: Host readiness projection, when this runtime has one.
/// - `observed_generation`: Catalog generation the response's model rows were
///   read from, or `None` when the service exposes no upstream catalog.
pub(super) fn attach_provider_readiness(
    response: &mut ModelLibraryResponse,
    readiness: Option<scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection>,
    observed_generation: Option<scriberig_app::ModelCatalogGeneration>,
) {
    let Some(observed_generation) = observed_generation else {
        return;
    };
    if let Some(rows) =
        readiness.and_then(|readiness| readiness.aligned_rows_for(response, &observed_generation))
    {
        response.provider_readiness = rows;
    }
}
