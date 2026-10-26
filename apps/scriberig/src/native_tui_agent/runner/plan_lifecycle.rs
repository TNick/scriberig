//! Runs durable plan lifecycle transitions through the native provider runtime.

use super::super::{NativePlanWorkerExecutor, NativeTuiAgentRunner};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, SessionBackend,
    SessionRequest,
};
use scriberig_core::EntityId;

/// Persists a plan transition and runs newly started work to its next idle point.
///
/// # Parameters
/// - `runner`: Native provider and model-library composition.
/// - `service`: Shared application service owning durable sessions.
/// - `context`: Trusted capabilities for this plan execution.
/// - `request`: Exact session, plan, and observed revision.
/// - `action`: Requested plan transition.
///
/// # Returns
/// The latest durable plan after the coordinator reaches idle.
///
/// # Errors
/// Returns lifecycle, provider, assignment, or persistence errors.
pub(super) fn run<B>(
    runner: &NativeTuiAgentRunner,
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &scriberig_app::PlanLifecycleRequest,
    action: scriberig_app::PlanLifecycleAction,
) -> Result<scriberig_session::Plan, ApplicationError>
where
    B: ApplicationBackend + Send + Sync + 'static,
    ApplicationService<B>: SessionBackend,
{
    let previous = SessionBackend::read_session(
        service,
        context,
        &SessionRequest {
            session: request.session,
        },
    )?;
    let previous_plan = previous.plans.iter().find(|plan| plan.id == request.plan);
    let was_running = previous_plan.is_some_and(|plan| {
        plan.status == scriberig_session::PlanStatus::Running && plan.coordinator_claim.is_some()
    });
    super::validation::validate_plan_provider(runner, previous_plan, action)?;
    let plan = SessionBackend::plan_lifecycle(service, context, request, action)?;
    if was_running
        || !matches!(
            action,
            scriberig_app::PlanLifecycleAction::Start | scriberig_app::PlanLifecycleAction::Resume
        )
    {
        return Ok(plan);
    }
    let claim = plan
        .coordinator_claim
        .as_ref()
        .ok_or_else(|| ApplicationError::Internal {
            message: "running plan has no coordinator claim".to_owned(),
        })?;
    {
        let mut active =
            runner
                .active_coordinators
                .lock()
                .map_err(|_| ApplicationError::Internal {
                    message: "plan coordinator guard is unavailable".to_owned(),
                })?;
        if !active.insert(plan.id) {
            return Ok(plan);
        }
    }
    let executor = NativePlanWorkerExecutor::with_upstream_runner(
        runner,
        service,
        context,
        request.session,
        std::sync::Arc::clone(&runner.project_store),
        claim.operation.clone(),
    );
    let upstream_models = service.upstream_models();
    let execution = scriberig_agent_runtime::coordinator::PlanCoordinator::run_until_idle(
        service,
        context,
        request.session,
        plan.id,
        claim.id,
        &executor,
        &upstream_models,
        &runner.model,
        4,
        EntityId::new,
    );
    if let Ok(mut active) = runner.active_coordinators.lock() {
        active.remove(&plan.id);
    }
    execution?;
    let latest = SessionBackend::read_session(
        service,
        context,
        &SessionRequest {
            session: request.session,
        },
    )?;
    latest
        .plans
        .into_iter()
        .find(|candidate| candidate.id == plan.id)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: plan.id.to_string(),
        })
}
