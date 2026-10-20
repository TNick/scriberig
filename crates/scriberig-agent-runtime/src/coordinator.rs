//! Drives durable plan workers through the shared application session boundary.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, SessionBackend,
    SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_session::{Branch, BranchResult, ExecutionActivity, Plan, PlanStatus, Session};

use crate::scheduler::{PlanScheduler, PlanWorkerExecutor, ScheduleTick, WorkerBatchResult};

/// Returns the durable outcome of one coordinator scheduling pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanAdvanceResult {
    /// Schedule observed before this pass dispatched workers.
    pub schedule: ScheduleTick,
    /// Worker outcomes committed by this pass.
    pub workers: WorkerBatchResult,
    /// Latest plan revision persisted after this pass.
    pub plan: Plan,
}

/// Owns one plan's sequential auto-advancement loop.
pub struct PlanCoordinator;

impl PlanCoordinator {
    /// Runs ready plan work until the plan has no immediately dispatchable node.
    ///
    /// Each batch acquires worker claims with optimistic persistence before
    /// entering the executor. Results are written to the session before plan
    /// dependency edges are fulfilled.
    ///
    /// - `service`: Shared application service owning durable session state.
    /// - `context`: Authorized application context for reads and mutations.
    /// - `session_id`: Durable session containing the plan.
    /// - `plan_id`: Plan to advance.
    /// - `coordinator_claim`: Exact claim held by this coordinator runtime.
    /// - `executor`: Provider-neutral worker executor.
    /// - `upstream_models`: Host-owned upstream model catalog.
    /// - `runtime_fallback`: Runtime model used when a node has no assignment.
    /// - `concurrency_budget`: Maximum workers in one batch.
    /// - `next_id`: Allocates stable branch and claim identities.
    ///
    /// One result per scheduling pass, including the final idle pass.
    ///
    /// # Returns
    /// One result per scheduling pass through the final idle pass.
    ///
    /// # Errors
    /// Returns authorization, stale-claim, persistence, assignment, or worker
    /// execution failures without silently retrying an active claim.
    // Service, execution policy, and identity allocation stay explicit here.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn run_until_idle<B: ApplicationBackend, E: PlanWorkerExecutor>(
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        session_id: EntityId,
        plan_id: EntityId,
        coordinator_claim: EntityId,
        executor: &E,
        upstream_models: &[rpi_ai::Model],
        runtime_fallback: &str,
        concurrency_budget: usize,
        mut next_id: impl FnMut() -> EntityId,
    ) -> Result<Vec<PlanAdvanceResult>, ApplicationError>
    where
        ApplicationService<B>: SessionBackend,
    {
        let mut passes = Vec::new();
        loop {
            let result = Self::advance_once(
                service,
                context,
                session_id,
                plan_id,
                coordinator_claim,
                executor,
                upstream_models,
                runtime_fallback,
                concurrency_budget,
                &mut next_id,
            )?;
            let dispatched = !result.schedule.dispatch.is_empty();
            let terminal = matches!(
                result.plan.status,
                PlanStatus::Completed | PlanStatus::Failed | PlanStatus::Cancelled
            );
            passes.push(result);
            if !dispatched || terminal {
                return Ok(passes);
            }
        }
    }

    /// Claims one dispatch batch, executes workers, then persists its results.
    ///
    /// - `service`: Shared application service owning durable session state.
    /// - `context`: Authorized application context for reads and mutations.
    /// - `session_id`: Durable session containing the plan.
    /// - `plan_id`: Plan to advance.
    /// - `coordinator_claim`: Exact claim held by this coordinator runtime.
    /// - `executor`: Provider-neutral worker executor.
    /// - `upstream_models`: Host-owned upstream model catalog.
    /// - `runtime_fallback`: Runtime model used when a node has no assignment.
    /// - `concurrency_budget`: Maximum workers in this batch.
    /// - `next_id`: Allocates stable branch and claim identities.
    ///
    /// The schedule, completed or failed workers, and durable latest plan.
    ///
    /// # Returns
    /// The schedule, worker outcomes, and latest durable plan.
    ///
    /// # Errors
    /// Returns authorization, stale-claim, persistence, assignment, or worker
    /// execution failures without silently retrying an active claim.
    // One coordinator tick keeps claim, execution, and persistence order visible.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn advance_once<B: ApplicationBackend, E: PlanWorkerExecutor>(
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        session_id: EntityId,
        plan_id: EntityId,
        coordinator_claim: EntityId,
        executor: &E,
        upstream_models: &[rpi_ai::Model],
        runtime_fallback: &str,
        concurrency_budget: usize,
        next_id: &mut impl FnMut() -> EntityId,
    ) -> Result<PlanAdvanceResult, ApplicationError>
    where
        ApplicationService<B>: SessionBackend,
    {
        let mut session = SessionBackend::read_session(
            service,
            context,
            &SessionRequest {
                session: session_id,
            },
        )?;
        let mut plan = get_plan(&session, plan_id)?.clone();
        require_coordinator(&plan, coordinator_claim)?;
        let results = branch_results(&session);
        let selection = session.execution.clone();
        let schedule = PlanScheduler::tick(&plan, &results, concurrency_budget, |_| true);
        if schedule.dispatch.is_empty() {
            return Ok(PlanAdvanceResult {
                schedule,
                workers: WorkerBatchResult {
                    completed: Vec::new(),
                    failed: Vec::new(),
                    results: Vec::new(),
                    model_request_evidence: Vec::new(),
                },
                plan,
            });
        }

        let expected_revision = plan.revision;
        let mut persisted_revision = None;
        let mut persistence_error = None;
        let workers = PlanScheduler::execute_dispatch_with_context_persisted(
            &mut plan,
            &schedule.dispatch,
            &results,
            executor,
            upstream_models,
            &selection,
            runtime_fallback,
            next_id,
            |claimed_plan, node, claim| {
                let mut worker_session = session.clone();
                ensure_worker_branches(&mut worker_session, claimed_plan)?;
                crate::plan_context::PlanWorkerContext::from_plan(
                    &worker_session,
                    claimed_plan,
                    node,
                    claim,
                )
            },
            |claimed| {
                let mut claimed_session = session.clone();
                ensure_worker_branches(&mut claimed_session, claimed)?;
                replace_plan(&mut claimed_session, claimed.clone())
                    .map_err(|error| error.to_string())?;
                match SessionBackend::compare_and_swap_plan(
                    service,
                    context,
                    &claimed_session,
                    plan_id,
                    expected_revision,
                ) {
                    Ok(()) => {
                        persisted_revision = Some(claimed.revision);
                        Ok(())
                    }
                    Err(error) => {
                        let message = error.to_string();
                        persistence_error = Some(error);
                        Err(message)
                    }
                }
            },
        );
        let workers = match workers {
            Ok(batch) => batch,
            Err(message) => match persistence_error {
                Some(error) => return Err(error),
                None => return Err(internal_error(message)),
            },
        };
        let persisted_revision = persisted_revision
            .ok_or_else(|| internal_error("scheduler returned without persisting worker claims"))?;

        session = SessionBackend::read_session(
            service,
            context,
            &SessionRequest {
                session: session_id,
            },
        )?;
        let observed = get_plan(&session, plan_id)?.clone();
        require_coordinator(&observed, coordinator_claim)?;
        if observed.revision != persisted_revision {
            return Err(revision_conflict(plan_id));
        }
        persist_worker_results(&mut session, &plan, &workers.results)?;
        for evidence in &workers.model_request_evidence {
            let branch = session
                .branches
                .iter_mut()
                .find(|branch| branch.id == evidence.branch)
                .ok_or_else(|| internal_error("worker request evidence has no claimed branch"))?;
            service.persist_plan_worker_model_requests(branch, evidence.requests.clone());
        }
        SessionBackend::save_session(service, context, &session)?;

        let latest = SessionBackend::read_session(
            service,
            context,
            &SessionRequest {
                session: session_id,
            },
        )?;
        let observed_plan = get_plan(&latest, plan_id)?;
        if observed_plan.revision != persisted_revision {
            return Err(revision_conflict(plan_id));
        }
        let latest_plan = plan.clone();
        let mut updated = latest;
        replace_plan(&mut updated, latest_plan.clone())?;
        SessionBackend::compare_and_swap_plan(
            service,
            context,
            &updated,
            plan_id,
            persisted_revision,
        )?;
        if latest_plan.status == PlanStatus::Completed
            && let Some(operation) = observed
                .coordinator_claim
                .as_ref()
                .and_then(|claim| claim.operation.as_ref())
        {
            service
                .operations()
                .transition(operation, scriberig_app::OperationStatus::Completed)?;
        }

        Ok(PlanAdvanceResult {
            schedule,
            workers,
            plan: latest_plan,
        })
    }
}

#[path = "coordinator_helpers.rs"]
mod coordinator_helpers;
use coordinator_helpers::{
    branch_results, ensure_worker_branches, get_plan, internal_error, persist_worker_results,
    replace_plan, require_coordinator, revision_conflict,
};
