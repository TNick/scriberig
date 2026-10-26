//! Reads one historical plan projection from an exact semantic snapshot.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, SessionBackend,
    SessionRequest,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{Plan, PlanNodeView, PlanView};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Reads compact plan and node views at the current or exact snapshot.
    ///
    /// The projection contains no plan revision history or full gate/evidence
    /// collections. Detailed records remain available through bounded reads.
    ///
    /// # Parameters
    /// - `self`: Application service owning session persistence.
    /// - `context`: Authorized request context.
    /// - `session`: Session containing the plan.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact plan-root snapshot, or `None` for current state.
    ///
    /// # Returns
    /// The compact plan view and its compact node views.
    ///
    /// # Errors
    /// Returns authorization, lookup, persistence, or projection errors.
    pub fn read_plan_views(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        plan: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<(PlanView, Vec<PlanNodeView>), ApplicationError> {
        super::super::service_sessions::authorize_plan_read(context, session)?;
        let mut session_state = None;
        if self.session_store.is_none() && snapshot.is_none() {
            session_state = Some(SessionBackend::read_session(
                self,
                context,
                &SessionRequest { session },
            )?);
        }
        let plan_state = if let Some(store) = &self.session_store {
            store
                .load_plan_projection(session, plan, snapshot)
                .map_err(|message| ApplicationError::Internal { message })?
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: match snapshot {
                        Some(snapshot) => format!("plan {plan} at snapshot {snapshot}"),
                        None => plan.to_string(),
                    },
                })?
        } else if let Some(snapshot) = snapshot {
            self.read_plan_at_snapshot(context, session, plan, snapshot)?
        } else {
            session_state
                .as_ref()
                .ok_or_else(|| ApplicationError::Internal {
                    message: "current session projection is unavailable".to_owned(),
                })?
                .plans
                .iter()
                .find(|candidate| candidate.id == plan)
                .cloned()
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: plan.to_string(),
                })?
        };
        let results = if snapshot.is_some() {
            None
        } else if let Some(store) = &self.session_store {
            if let Some(results) = store
                .load_plan_branch_results(session)
                .map_err(|message| ApplicationError::Internal { message })?
            {
                Some(results)
            } else {
                session_state = Some(SessionBackend::read_session(
                    self,
                    context,
                    &SessionRequest { session },
                )?);
                session_state.as_ref().map(|state| {
                    state
                        .branches
                        .iter()
                        .filter_map(|branch| branch.result.clone())
                        .collect()
                })
            }
        } else {
            session_state.as_ref().map(|state| {
                state
                    .branches
                    .iter()
                    .filter_map(|branch| branch.result.clone())
                    .collect()
            })
        };
        let rollups = if let (Some(store), Some(source_snapshot)) =
            (&self.session_store, plan_state.source_snapshot)
        {
            match store
                .load_plan_rollups(session, plan, source_snapshot)
                .map_err(|message| ApplicationError::Internal { message })?
            {
                Some(rollups) => rollups,
                None => plan_state.node_rollups().map_err(view_error)?,
            }
        } else {
            plan_state.node_rollups().map_err(view_error)?
        };
        let plan_view = plan_state.plan_view(snapshot).map_err(view_error)?;
        let node_views = plan_state
            .nodes
            .iter()
            .map(|node| {
                plan_state
                    .node_view(node.id, snapshot, results.as_deref(), &rollups)
                    .map_err(view_error)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((plan_view, node_views))
    }

    /// Reads one plan projection from the exact semantic snapshot requested.
    ///
    /// The caller must hold the normal session-read capability. Durable stores
    /// resolve the projection from the plan root's history without hydrating
    /// unrelated revisions. In-memory services can serve only their current
    /// projection when it carries the same snapshot.
    ///
    /// # Parameters
    /// - `self`: Application service owning session persistence.
    /// - `context`: Authorized request context.
    /// - `session`: Session containing the plan.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact universal snapshot to resolve.
    ///
    /// # Returns
    /// The plan projection at the requested snapshot.
    ///
    /// # Errors
    /// Returns authorization, lookup, persistence, or unsupported-read errors.
    pub fn read_plan_at_snapshot(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        plan: EntityId,
        snapshot: SnapshotId,
    ) -> Result<Plan, ApplicationError> {
        super::super::service_sessions::authorize_plan_read(context, session)?;
        if let Some(store) = &self.session_store {
            return store
                .load_plan_projection(session, plan, Some(snapshot))
                .map_err(|message| ApplicationError::Internal { message })?
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: format!("plan {plan} at snapshot {snapshot}"),
                });
        }
        let current = SessionBackend::read_session(self, context, &SessionRequest { session })?;
        if !current.plans.iter().any(|candidate| candidate.id == plan) {
            return Err(ApplicationError::NotFound {
                subject: plan.to_string(),
            });
        }
        let current_plan = current
            .plans
            .into_iter()
            .find(|candidate| candidate.id == plan)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: plan.to_string(),
            })?;
        if current_plan.source_snapshot == Some(snapshot) {
            Ok(current_plan)
        } else {
            Err(ApplicationError::UnsupportedOperation {
                operation: "historical plan snapshot read".to_owned(),
            })
        }
    }
}

/// Converts a shared plan-view assembly failure into an application error.
///
/// - `error`: The domain error returned by the view builder.
///
/// # Returns
/// A bounded internal error that does not expose storage details.
fn view_error(error: impl std::fmt::Debug) -> ApplicationError {
    ApplicationError::Internal {
        message: format!("plan view assembly failed: {error:?}"),
    }
}
