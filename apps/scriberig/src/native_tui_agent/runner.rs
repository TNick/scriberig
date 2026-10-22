//! Implements native TUI agent and plan lifecycle execution.

use std::sync::Arc;

#[path = "runner/completion.rs"]
mod completion;
use completion::complete_accepted_result;
#[path = "runner/validation.rs"]
mod validation;
use validation::validate_plan_provider;
#[path = "runner/summary_activity.rs"]
mod summary_activity;
use summary_activity::ReasoningSummaryActivity;

use super::{
    NativePlanWorkerExecutor, NativeTuiAgentRunner, previous_workspace_selection,
    selected_runtime_model,
};
use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;

impl<B: ApplicationBackend + Sync> scriberig_tui::AgentCommandRunner<B> for NativeTuiAgentRunner {
    /// Reads provider-schema-backed properties for the selected model.
    ///
    /// # Parameters
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    ///
    /// # Returns
    /// Upstream property descriptors for the local settings surface.
    /// # Errors
    /// Returns a safe error if the model schema is unavailable.
    fn model_properties(
        &self,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<rpi_model_runtime::ModelPropertyDescriptor>, ApplicationError> {
        NativeTuiAgentRunner::model_properties(self, provider_id, model_id).map_err(|_| {
            ApplicationError::UnsupportedOperation {
                operation: "upstream model property settings".to_owned(),
            }
        })
    }

    /// Validates a model property through upstream and updates the live catalog.
    ///
    /// # Parameters
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    /// - `key`: Provider-declared property key.
    /// - `value`: User value passed to upstream validation.
    ///
    /// # Returns
    /// Success after the config and live catalog are updated.
    /// # Errors
    /// Returns a safe validation or config error.
    fn set_model_property(
        &self,
        provider_id: &str,
        model_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), ApplicationError> {
        NativeTuiAgentRunner::set_model_property(self, provider_id, model_id, key, value).map_err(
            |_| ApplicationError::SchemaViolation {
                message: "upstream model property update was rejected".to_owned(),
            },
        )
    }

    /// Resolves safe per-model credential status from upstream and host policy.
    ///
    /// # Returns
    /// Credential readiness rows without resolved credential material.
    fn provider_readiness(&self) -> Vec<scriberig_protocol::ProviderReadiness> {
        super::provider_readiness::project(self)
    }

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
        let executor = NativePlanWorkerExecutor::with_upstream_runner(
            self,
            service,
            context,
            request.session,
            std::sync::Arc::clone(&self.project_store),
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

    #[expect(
        clippy::too_many_lines,
        reason = "This coordinates one accepted root branch and validates continuation provenance before caching it."
    )]
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
        let selected = selected_runtime_model(service, context, session, &self.model)?;
        let thinking_level = selected.thinking_level;
        let connection = selected.connection;
        let cancelled = service.operations().cancellation_flag(&handle.operation)?;
        let cached = self
            .continuations
            .lock()
            .ok()
            .and_then(|mut entries| entries.remove(&handle.root_branch));
        let runtime_context = selection.as_ref().map_or_else(
            || context.clone(),
            |selection| context.clone().with_snapshot(selection.snapshot),
        );
        let upstream_provider = match self.upstream_provider_for(&selected.model, &connection) {
            Ok(provider) => provider,
            Err(error) => {
                let message = error.to_string();
                service.fail_local_agent(handle, &message)?;
                return Err(ApplicationError::UnsupportedOperation { operation: message });
            }
        };
        let Some((upstream, upstream_model, request_secret)) = upstream_provider else {
            let message = "configure an upstream provider before starting agent work";
            service.fail_local_agent(handle, message)?;
            return Err(ApplicationError::UnsupportedOperation {
                operation: message.to_owned(),
            });
        };
        let provider_id = upstream.id().to_owned();
        let context_fingerprint = super::upstream_execution::completed_root_context_fingerprint(
            service,
            &runtime_context,
            session,
            handle.root_branch,
            "",
            selection.as_ref(),
        )
        .ok();
        let continuation = cached.and_then(|entry| {
            let exact_identity = entry.provider_id == upstream.id()
                && entry.model == upstream_model
                && entry.profile == connection
                && entry.thinking_level == thinking_level.unwrap_or_default();
            let exact_context = context_fingerprint
                .as_ref()
                .is_some_and(|fingerprint| fingerprint == &entry.provider_context_fingerprint);
            (exact_identity
                && exact_context
                && upstream.supports_response_continuation()
                && entry.handle.is_scoped_to(upstream.id(), &upstream_model.id))
            .then_some((entry.handle, entry.provider_context_fingerprint))
        });
        let summary_activity = ReasoningSummaryActivity::new(
            service.operations().clone(),
            handle.operation.clone(),
            handle.root_branch.to_string(),
        );
        let result = super::upstream_execution::execute_root_branch_with_continuation(
            service,
            &runtime_context,
            session,
            handle.root_branch,
            &handle.prompt,
            selection.as_ref(),
            Arc::clone(&self.project_store),
            Arc::clone(&upstream),
            upstream_model.clone(),
            request_secret,
            thinking_level,
            &cancelled,
            Some(connection.clone()),
            None,
            Some(summary_activity.callback()),
            continuation,
        );
        summary_activity.settle();
        let result = match result {
            Ok(result) => result,
            Err(failure) => {
                let message = failure.failure.error.to_string();
                service.fail_local_agent_with_evidence(
                    handle,
                    &message,
                    failure.failure.model_requests,
                    failure.tool_evidence,
                )?;
                return Err(ApplicationError::OperationFailed {
                    operation: handle.operation.to_string(),
                    message,
                });
            }
        };
        let continuation = result.continuation;
        let completed =
            complete_accepted_result(service, handle, session, selection.as_ref(), result.result)?;
        if !cancelled.load(std::sync::atomic::Ordering::Acquire)
            && upstream.supports_response_continuation()
            && let Some(continuation) = continuation
            && continuation.is_scoped_to(upstream.id(), &upstream_model.id)
            && let Ok(fingerprint) = super::upstream_execution::completed_root_context_fingerprint(
                service,
                &runtime_context,
                session,
                handle.root_branch,
                "",
                selection.as_ref(),
            )
            && !cancelled.load(std::sync::atomic::Ordering::Acquire)
            && let Ok(mut entries) = self.continuations.lock()
        {
            entries.insert(
                handle.root_branch,
                super::RootContinuationCacheEntry {
                    provider_id,
                    model: upstream_model,
                    profile: connection,
                    thinking_level: thinking_level.unwrap_or_default(),
                    provider_context_fingerprint: fingerprint,
                    handle: continuation,
                },
            );
        }
        Ok(completed)
    }
}
