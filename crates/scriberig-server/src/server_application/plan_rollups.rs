//! Loads shared rollups for each snapshot-bound plan in a session.

use super::super::{ApplicationContext, ApplicationError};
use scriberig_app::{ApplicationBackend, ApplicationService};
use scriberig_core::EntityId;
use scriberig_session::{PlanNodeRollup, Session};
use std::collections::BTreeMap;

/// Reads exact-snapshot rollups through the shared application boundary.
///
/// # Parameters
/// - `service`: Shared application service.
/// - `context`: Trusted request context.
/// - `session`: Loaded session containing canonical plan projections.
///
/// # Returns
/// Rollups indexed first by plan and then stable node identity.
///
/// # Errors
/// Returns authorization, not-found, or persistence failures.
pub(super) fn read<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: &Session,
) -> Result<BTreeMap<EntityId, BTreeMap<EntityId, PlanNodeRollup>>, ApplicationError> {
    let mut rollups = BTreeMap::new();
    for plan in &session.plans {
        if let Some(snapshot) = plan.source_snapshot {
            rollups.insert(
                plan.id,
                service.read_plan_rollups(context, session.id, plan.id, snapshot)?,
            );
        }
    }
    Ok(rollups)
}
