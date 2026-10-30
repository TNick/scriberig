//! Names plan roots and adapts storage failures for the semantic plan repository.

use scriberig_core::{EntityId, RootName};
use scriberig_session::PlanStoreError;

/// Names the canonical universal root for one session plan.
///
/// # Parameters
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity.
///
/// # Returns
/// The deterministic plan root name.
///
/// # Errors
/// Returns an error if the generated name is invalid.
pub(crate) fn plan_root(session: EntityId, plan: EntityId) -> Result<RootName, String> {
    RootName::new(format!("scriberig-plan-{session}-{plan}"))
        .map_err(|error| format!("invalid plan root name: {error:?}"))
}

/// Converts a storage diagnostic to the plan repository error type.
///
/// # Parameters
/// - `message`: Bounded persistence diagnostic.
///
/// # Returns
/// A typed plan persistence error.
pub(super) fn plan_persistence(message: impl Into<String>) -> PlanStoreError {
    PlanStoreError::Persistence(message.into())
}
