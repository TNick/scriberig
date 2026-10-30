//! Builds deterministic upstream provider inputs for the semantic project flow.

use rpi_ai::providers::faux::{FauxProvider, FauxScript};
use scriberig_core::{EntityId, SnapshotId};

/// Creates one deterministic upstream Faux provider and its exact model.
///
/// - `entity`: Selected semantic entity for the inspect call.
/// - `snapshot`: Exact snapshot expected by semantic tools.
/// - `replacement`: Deterministic candidate expression.
///
/// # Returns
/// A deterministic upstream Faux provider and model.
pub(super) fn faux_provider(
    entity: EntityId,
    snapshot: SnapshotId,
    replacement: &str,
) -> (std::sync::Arc<dyn rpi_ai::Provider>, rpi_ai::Model) {
    let provider = FauxProvider::new(
        FauxScript::new()
            .with_tool_call(
                "inspect",
                serde_json::json!({
                    "entity": entity.to_string(),
                    "snapshot": snapshot.to_string(),
                    "detail": "detailed"
                }),
            )
            .with_tool_call("project_propose", serde_json::json!({"text": replacement}))
            .with_text("Candidate ready for review."),
    );
    let model = provider.default_model().clone();
    (provider, model)
}
