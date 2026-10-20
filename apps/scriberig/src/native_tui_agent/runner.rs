//! Implements native TUI agent and plan lifecycle execution.

use super::{
    NativePlanWorkerExecutor, NativeTuiAgentRunner, execute_agent, previous_workspace_selection,
    selected_runtime_model, selection_handles, workspace_from_result,
};
use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;

/// Validates provider availability before a plan can begin automatic work.
///
/// Integration-only nodes may merge dependency candidates without a model
/// provider. Every other pending node requires a configured provider adapter.
///
/// - `runner`: Configured provider adapters for the current native runtime.
/// - `plan`: Current durable plan state, when it exists.
/// - `action`: Requested lifecycle transition.
///
/// Unit when provider configuration is sufficient for the transition.
/// # Returns
/// Whether the plan may proceed to lifecycle mutation.
/// # Errors
/// Returns an unsupported-operation error when worker execution lacks a provider.
fn validate_plan_provider(
    runner: &NativeTuiAgentRunner,
    plan: Option<&scriberig_session::Plan>,
    action: scriberig_app::PlanLifecycleAction,
) -> Result<(), ApplicationError> {
    let begins_execution = matches!(
        action,
        scriberig_app::PlanLifecycleAction::Start | scriberig_app::PlanLifecycleAction::Resume
    );
    let needs_provider = plan.is_some_and(|plan| {
        plan.nodes.iter().any(|node| {
            matches!(
                node.status,
                scriberig_session::PlanNodeStatus::Planned
                    | scriberig_session::PlanNodeStatus::Ready
            ) && node
                .expected_result
                .as_ref()
                .is_none_or(|expected| expected.kind != "project_candidate_merge")
        })
    });
    if begins_execution && needs_provider && runner.providers.is_empty() {
        return Err(ApplicationError::UnsupportedOperation {
            operation: "configure a provider connection before starting plan work".to_owned(),
        });
    }
    Ok(())
}

impl<B: ApplicationBackend + Sync> scriberig_tui::AgentCommandRunner<B> for NativeTuiAgentRunner {
    /// Persists a plan transition and runs newly started work to its next idle point.
    ///
    /// Start and resume transitions claim one durable coordinator before this
    /// method starts workers. Repeated start calls do not launch a second local
    /// loop for the same plan.
    ///
    /// - `self`: Native provider and model-library composition.
    /// - `service`: Shared application service owning durable sessions.
    /// - `context`: Trusted capabilities for this plan execution.
    /// - `request`: Exact session, plan, and observed revision.
    /// - `action`: Requested plan transition.
    ///
    /// The latest durable plan after the coordinator reaches idle.
    ///
    /// Returns lifecycle, provider, assignment, or persistence errors.
    /// # Returns
    /// The latest authoritative plan.
    /// # Errors
    /// Returns an error when the lifecycle operation or worker execution fails.
    fn plan_lifecycle(
        &self,
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        request: &scriberig_app::PlanLifecycleRequest,
        action: scriberig_app::PlanLifecycleAction,
    ) -> Result<scriberig_session::Plan, ApplicationError>
    where
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
            plan.status == scriberig_session::PlanStatus::Running
                && plan.coordinator_claim.is_some()
        });
        validate_plan_provider(self, previous_plan, action)?;
        let plan = SessionBackend::plan_lifecycle(service, context, request, action)?;
        if was_running
            || !matches!(
                action,
                scriberig_app::PlanLifecycleAction::Start
                    | scriberig_app::PlanLifecycleAction::Resume
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
                self.active_coordinators
                    .lock()
                    .map_err(|_| ApplicationError::Internal {
                        message: "plan coordinator guard is unavailable".to_owned(),
                    })?;
            if !active.insert(plan.id) {
                return Ok(plan);
            }
        }
        let executor = NativePlanWorkerExecutor::new(
            service,
            context,
            request.session,
            &self.providers,
            std::sync::Arc::clone(&self.project_store),
            claim.operation.clone(),
        );
        let execution = scriberig_agent_runtime::coordinator::PlanCoordinator::run_until_idle(
            service,
            context,
            request.session,
            plan.id,
            claim.id,
            &executor,
            &self.model_library,
            &self.model,
            4,
            EntityId::new,
        );
        if let Ok(mut active) = self.active_coordinators.lock() {
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

    /// Executes one TUI agent request through the trusted shared runtime.
    ///
    /// - `self`: Native provider and project-tool composition.
    /// - `service`: Shared application service owning durable sessions.
    /// - `context`: Trusted capabilities and semantic/bootstrap scope.
    /// - `session`: Session receiving the user instruction.
    /// - `branch`: Optional branch receiving the user instruction.
    /// - `prompt`: User-authored instruction.
    ///
    /// Stable operation, session, and branch handles after runtime completion.
    ///
    /// Returns an authorization, provider, tool, or application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit(
        &self,
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_tui::AgentHandle, ApplicationError> {
        let selection = previous_workspace_selection(service, context, session)?;
        let request = AgentCommandRequest {
            session,
            branch,
            anchor: None,
            prompt: prompt.to_owned(),
            selection: selection.clone(),
        };
        let handle = service.begin_agent_execution(context, &request)?;
        self.execute_accepted(service, context, &handle)
    }

    /// Runs one already persisted operation through the trusted model runtime.
    ///
    /// - `self`: Native provider and project-tool composition.
    /// - `service`: Shared application service owning durable sessions.
    /// - `context`: Trusted capabilities and semantic/bootstrap scope.
    /// - `handle`: Accepted operation and branch created by the application.
    ///
    /// Stable operation, session, and branch handles after runtime completion.
    ///
    /// Returns an authorization, provider, tool, or application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute_accepted(
        &self,
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        handle: &scriberig_app::LocalAgentHandle,
    ) -> Result<scriberig_tui::AgentHandle, ApplicationError> {
        let session = handle.session;
        let selection = handle.selection.clone().map_or_else(
            || previous_workspace_selection(service, context, session),
            |value| Ok(Some(value)),
        )?;
        let (model, thinking_level, connection) =
            selected_runtime_model(service, context, session, &self.model)?;
        let cancelled = service.operations().cancellation_flag(&handle.operation)?;
        let Some(provider) = self.providers.get(&connection) else {
            let message = "configure a provider connection before starting agent work";
            service.fail_local_agent(handle, message)?;
            return Err(ApplicationError::UnsupportedOperation {
                operation: message.to_owned(),
            });
        };
        let runtime_context = selection.as_ref().map_or_else(
            || context.clone(),
            |selection| context.clone().with_snapshot(selection.snapshot),
        );
        let result = execute_agent(
            provider.clone(),
            &model,
            thinking_level.as_deref(),
            service,
            &self.project_store,
            &runtime_context,
            session,
            handle.root_branch,
            handle.operation.clone(),
            &handle.prompt,
            selection.as_ref(),
            &cancelled,
        );
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let message = error.to_string();
                service.fail_local_agent(handle, &message)?;
                return Err(ApplicationError::OperationFailed {
                    operation: handle.operation.to_string(),
                    message,
                });
            }
        };
        let workspace = workspace_from_result(&result)?;
        let mut result_handles = crate::child_runtime::compact_result_handles(&result);
        let workspace_handles = workspace.as_ref().map_or_else(
            || selection_handles(selection.as_ref()),
            |workspace| {
                vec![
                    format!("project:{}", workspace.project),
                    format!("snapshot:{}", workspace.snapshot),
                    format!("root:{}", workspace.root),
                    format!("manifest:{}", workspace.manifest_id),
                ]
            },
        );
        for handle in workspace_handles {
            if !result_handles.contains(&handle) {
                result_handles.push(handle);
            }
        }
        service.complete_local_agent_with_evidence(
            handle,
            result.output,
            result_handles,
            result.tool_evidence,
            result.model_requests,
        )?;
        Ok(scriberig_tui::AgentHandle {
            operation: handle.operation.clone(),
            session,
            branch: Some(handle.root_branch),
            workspace,
            failure: None,
        })
    }
}
