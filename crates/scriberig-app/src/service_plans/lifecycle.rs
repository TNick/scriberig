//! Implements validated durable plan application operations.
use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    PlanLifecycleRequest, SessionBackend,
};
use scriberig_session::{Plan, PlanStatus};

use super::validation::{invalid_plan, revision_conflict};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Starts automatic advancement for one observed plan revision.
    ///
    /// - `self`: Application service owning durable plan state.
    /// - `context`: Authorized caller context.
    /// - `request`: Session, plan, and observed revision.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns a revision conflict, invalid transition, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn start_plan(
        &self,
        context: &ApplicationContext,
        request: &PlanLifecycleRequest,
    ) -> Result<Plan, ApplicationError> {
        self.mutate_plan(context, request, PlanStatus::Running)
    }

    /// Pauses dispatch for one observed plan revision.
    ///
    /// - `self`: Application service.
    /// - `context`: Authorized caller context.
    /// - `request`: Session, plan, and observed revision.
    ///
    /// The authoritative paused plan.
    ///
    /// Returns a revision conflict, invalid transition, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn pause_plan(
        &self,
        context: &ApplicationContext,
        request: &PlanLifecycleRequest,
    ) -> Result<Plan, ApplicationError> {
        self.mutate_plan(context, request, PlanStatus::Paused)
    }

    /// Resumes dispatch for one observed plan revision.
    ///
    /// - `self`: Application service.
    /// - `context`: Authorized caller context.
    /// - `request`: Session, plan, and observed revision.
    ///
    /// The authoritative running plan.
    ///
    /// Returns a revision conflict, invalid transition, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn resume_plan(
        &self,
        context: &ApplicationContext,
        request: &PlanLifecycleRequest,
    ) -> Result<Plan, ApplicationError> {
        self.mutate_plan(context, request, PlanStatus::Running)
    }

    /// Stops automatic advancement for one observed plan revision.
    ///
    /// - `self`: Application service.
    /// - `context`: Authorized caller context.
    /// - `request`: Session, plan, and observed revision.
    ///
    /// The authoritative cancelled plan.
    ///
    /// Returns a revision conflict, invalid transition, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn stop_plan(
        &self,
        context: &ApplicationContext,
        request: &PlanLifecycleRequest,
    ) -> Result<Plan, ApplicationError> {
        self.mutate_plan(context, request, PlanStatus::Cancelled)
    }

    /// Persists final completion after every node result is available.
    ///
    /// - `self`: Application service owning durable plan state.
    /// - `context`: Authorized caller context.
    /// - `request`: Session, plan, and observed revision.
    ///
    /// The completed plan with its coordinator claim released.
    ///
    /// Returns a revision conflict, incomplete-work error, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_plan(
        &self,
        context: &ApplicationContext,
        request: &PlanLifecycleRequest,
    ) -> Result<Plan, ApplicationError> {
        let mut session = self.read_session(
            context,
            &crate::SessionRequest {
                session: request.session,
            },
        )?;
        let plan = session
            .plans
            .iter_mut()
            .find(|plan| plan.id == request.plan)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: request.plan.to_string(),
            })?;
        if plan.revision != request.expected_revision {
            return Err(revision_conflict(request.plan));
        }
        let operation = plan
            .coordinator_claim
            .as_ref()
            .and_then(|claim| claim.operation.clone());
        let results = session
            .branches
            .iter()
            .filter_map(|branch| branch.result.clone())
            .collect::<Vec<_>>();
        if !plan
            .complete_if_satisfied(&results)
            .map_err(|error| invalid_plan(format!("plan completion failed: {error:?}")))?
        {
            return Err(invalid_plan("plan still has unresolved node results"));
        }
        let completed = plan.clone();
        self.compare_and_swap_plan(context, &session, request.plan, request.expected_revision)?;
        self.operations.publish_plan_activity(
            request.session,
            request.plan,
            None,
            scriberig_session::SessionActivityKind::CoordinatorChanged,
            scriberig_session::SessionActivityState::Completed,
            "Plan coordinator completed",
        );
        if let Some(operation) = operation {
            let _ = self
                .operations
                .transition(&operation, crate::OperationStatus::Completed);
        }
        Ok(completed)
    }

    /// Applies one optimistic plan lifecycle update and persists the session.
    ///
    /// - `self`: Application service.
    /// - `context`: Authorized caller context.
    /// - `request`: Session, plan, and observed revision.
    /// - `status`: Requested lifecycle state.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns conflict, invalid transition, lookup, or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    #[allow(clippy::too_many_lines)]
    fn mutate_plan(
        &self,
        context: &ApplicationContext,
        request: &PlanLifecycleRequest,
        status: PlanStatus,
    ) -> Result<Plan, ApplicationError> {
        let mut session = self.read_session(
            context,
            &crate::SessionRequest {
                session: request.session,
            },
        )?;
        let plan = session
            .plans
            .iter_mut()
            .find(|plan| plan.id == request.plan)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: request.plan.to_string(),
            })?;
        if plan.revision != request.expected_revision {
            return Err(revision_conflict(request.plan));
        }
        let previous_coordinator_operation = plan
            .coordinator_claim
            .as_ref()
            .and_then(|claim| claim.operation.clone());
        if status == PlanStatus::Running
            && plan.status == PlanStatus::Running
            && plan.coordinator_claim.is_some()
        {
            return Ok(plan.clone());
        }
        let coordinator_operation = if status == PlanStatus::Running {
            let sequence = self
                .next_agent_handle
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let operation = scriberig_core::OperationId::new(format!(
                "plan-coordinator-{sequence}"
            ))
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
            Some(operation)
        } else {
            None
        };
        if status == PlanStatus::Running && plan.coordinator_claim.is_none() {
            let owner = context
                .correlation_id
                .as_deref()
                .or_else(|| {
                    context
                        .principal
                        .as_ref()
                        .map(|principal| principal.id.as_str())
                })
                .unwrap_or("application-runtime");
            plan.start_with_coordinator_claim(
                self.next_entity(0xC3),
                owner,
                coordinator_operation.clone(),
            )
            .map_err(|error| invalid_plan(format!("coordinator start failed: {error:?}")))?;
            if let Some(operation) = &coordinator_operation {
                self.operations
                    .start(operation.clone(), crate::OperationKind::Plan)?;
                if let Err(error) = self
                    .operations
                    .transition(operation, crate::OperationStatus::Running)
                {
                    let _ = self.operations.cancel(operation);
                    return Err(error);
                }
            }
        } else {
            plan.transition(status)
                .map_err(|error| invalid_plan(format!("invalid plan transition: {error:?}")))?;
        }
        let updated = plan.clone();
        if let Err(error) =
            self.compare_and_swap_plan(context, &session, request.plan, request.expected_revision)
        {
            if let Some(operation) = &coordinator_operation {
                let _ = self.operations.cancel(operation);
            }
            return Err(error);
        }
        if status == PlanStatus::Paused || status == PlanStatus::Cancelled {
            if let Some(operation) = previous_coordinator_operation {
                let _ = self.operations.cancel(&operation);
            }
        }
        self.operations.publish_plan_activity(
            request.session,
            request.plan,
            None,
            scriberig_session::SessionActivityKind::CoordinatorChanged,
            match status {
                PlanStatus::Failed => scriberig_session::SessionActivityState::Failed,
                PlanStatus::Cancelled => scriberig_session::SessionActivityState::Cancelled,
                _ => scriberig_session::SessionActivityState::Completed,
            },
            match status {
                PlanStatus::Running => "Plan coordinator started",
                PlanStatus::Paused => "Plan coordinator paused",
                PlanStatus::Cancelled => "Plan coordinator cancelled",
                PlanStatus::Failed => "Plan coordinator failed",
                PlanStatus::Draft | PlanStatus::Planned | PlanStatus::Completed => {
                    "Plan coordinator changed"
                }
            },
        );
        Ok(updated)
    }
}
