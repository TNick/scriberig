//! Reads snapshot-bound plan rollups through the authorized session boundary.

use super::service_sessions::authorize_session_read;
use super::{ApplicationBackend, ApplicationService};
use crate::{ApplicationContext, ApplicationError, SessionRequest};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::PlanNodeRollup;
use std::collections::BTreeMap;

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Reads rollups from a valid cache or exact canonical plan state.
    ///
    /// # Parameters
    /// - `context`: Authorized caller context.
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact semantic snapshot to resolve.
    ///
    /// # Returns
    /// Effective statuses and subtree aggregates for each plan node.
    ///
    /// # Errors
    /// Returns authorization, not-found, or persistence failures.
    pub fn read_plan_rollups(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        plan: EntityId,
        snapshot: SnapshotId,
    ) -> Result<BTreeMap<EntityId, PlanNodeRollup>, ApplicationError> {
        read(self, context, session, plan, snapshot)
    }
}

/// Reads a valid cache or evaluates rollups from the exact canonical plan.
///
/// # Parameters
/// - `service`: Application service owning session storage.
/// - `context`: Authorized caller context.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact semantic snapshot to resolve.
///
/// # Returns
/// Effective statuses and subtree aggregates for each plan node.
///
/// # Errors
/// Returns authorization, not-found, or persistence failures.
pub(super) fn read<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
) -> Result<BTreeMap<EntityId, PlanNodeRollup>, ApplicationError> {
    authorize_session_read(context, &SessionRequest { session })?;
    if let Some(store) = &service.session_store {
        return store
            .load_plan_rollups_at_snapshot(session, plan, snapshot)
            .map_err(|message| ApplicationError::Internal { message })?
            .ok_or_else(|| ApplicationError::NotFound {
                subject: format!("plan snapshot {snapshot}"),
            });
    }
    let session_value = <ApplicationService<B> as crate::SessionBackend>::read_session(
        service,
        context,
        &SessionRequest { session },
    )?;
    let plan_value = session_value
        .plans
        .iter()
        .find(|value| value.id == plan && value.source_snapshot == Some(snapshot))
        .ok_or_else(|| ApplicationError::NotFound {
            subject: format!("plan snapshot {snapshot}"),
        })?;
    plan_value
        .node_rollups()
        .map_err(|error| ApplicationError::Internal {
            message: format!("plan rollup evaluation failed: {error:?}"),
        })
}
