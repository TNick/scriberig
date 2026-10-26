//! Implements native TUI agent and plan lifecycle execution.

use std::sync::Arc;

#[path = "runner/completion.rs"]
mod completion;
use completion::complete_accepted_result;
#[path = "runner/summary_activity.rs"]
mod summary_activity;
#[path = "runner/validation.rs"]
mod validation;
use summary_activity::ReasoningSummaryActivity;
#[path = "runner/plan_lifecycle.rs"]
mod plan_lifecycle;
#[path = "runner/plugin_command_logging.rs"]
mod plugin_command_logging;
#[path = "runner/plugin_commands.rs"]
mod plugin_commands;

use super::{NativeTuiAgentRunner, previous_workspace_selection, selected_runtime_model};
use scriberig_agent_runtime::continuation::{
    CompletedContinuation, ContinuationBinding, ContinuationOutcome,
};
use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService, SessionBackend,
};
use scriberig_core::EntityId;

impl NativeTuiAgentRunner {
    /// Returns the upstream mailbox used by isolated plugin UI actions.
    ///
    /// # Returns
    /// A clone sharing this runner's component registry.
    pub(crate) fn extension_component_mailbox(&self) -> rpi_extensions::ExtensionStatusMailbox {
        self.extension_component_mailbox.clone()
    }
}

impl<B: ApplicationBackend + Send + Sync + 'static> scriberig_tui::AgentCommandRunner<B>
    for NativeTuiAgentRunner
{
    /// Returns allowlisted command descriptors from the upstream worker.
    ///
    /// # Returns
    /// The command descriptors currently admitted by the isolated worker.
    fn plugin_commands(&self) -> Vec<rpi_extensions::isolated::WorkerCommandSpec> {
        plugin_commands::plugin_commands(self)
    }

    /// Invokes one allowlisted command through its upstream isolated worker.
    ///
    /// - `command`: Exact normalized command identity.
    /// - `arguments`: Raw text following the command name.
    ///
    /// # Returns
    /// The structured plugin response.
    ///
    /// # Errors
    /// Returns a safe error when the command is unavailable or execution fails.
    fn invoke_plugin_command(
        &self,
        command: &str,
        arguments: &str,
    ) -> Result<serde_json::Value, String> {
        plugin_commands::invoke_plugin_command(self, command, arguments)
    }

    /// Continues an allowlisted command with a structured upstream UI action.
    ///
    /// - `command`: Exact normalized command identity.
    /// - `arguments`: Raw text following the command name.
    /// - `action`: Structured selection or text submitted to the plugin.
    ///
    /// # Returns
    /// The next structured plugin response.
    ///
    /// # Errors
    /// Returns a safe error when the command is unavailable or execution fails.
    fn invoke_plugin_command_action(
        &self,
        command: &str,
        arguments: &str,
        action: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        plugin_commands::invoke_plugin_command_action(self, command, arguments, action)
    }

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

    /// Resolves secret-free provider credential readiness.
    ///
    /// # Returns
    /// Readiness rows paired with their catalog generation.
    fn provider_readiness(
        &self,
    ) -> Option<scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection> {
        Some(super::provider_readiness::project(self))
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
        plan_lifecycle::run(self, service, context, request, action)
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
        if let Err(error) = self.refresh_provider_packages() {
            let message = error.to_string();
            service.fail_local_agent(handle, &message)?;
            return Err(ApplicationError::UnsupportedOperation { operation: message });
        }
        let selected = selected_runtime_model(service, context, session, &self.model)?;
        let profile = selected.profile;
        let thinking_level = selected.thinking_level;
        let connection = selected.connection;
        let cancelled = service.operations().cancellation_flag(&handle.operation)?;
        let runtime_context = selection.as_ref().map_or_else(
            || context.clone(),
            |selection| context.clone().with_snapshot(selection.snapshot),
        );
        let upstream_provider = match self.upstream_provider_for(&selected.model, &connection) {
            Ok(provider) => provider,
            Err(error) => {
                self.continuations.discard(handle.root_branch);
                let message = error.to_string();
                service.fail_local_agent(handle, &message)?;
                return Err(ApplicationError::UnsupportedOperation { operation: message });
            }
        };
        let Some((upstream, upstream_model, request_secret)) = upstream_provider else {
            self.continuations.discard(handle.root_branch);
            let message = "configure an upstream provider before starting agent work";
            service.fail_local_agent(handle, message)?;
            return Err(ApplicationError::UnsupportedOperation {
                operation: message.to_owned(),
            });
        };
        // The submitted human turn is already durable when this candidate is
        // compiled, so the prompt must be supplied for the compiler to omit it.
        // That makes this fingerprint name the completed lineage, which is what
        // the post-completion binding below records with nothing to omit.
        let candidate_fingerprint = super::upstream_execution::completed_root_context_fingerprint(
            service,
            &runtime_context,
            session,
            handle.root_branch,
            &handle.prompt,
            selection.as_ref(),
            &profile,
        )
        .ok();
        // The handle is consumed before the attempt, so a divergent, failed, or
        // cancelled turn cannot leave it reusable.
        let continuation = candidate_fingerprint.and_then(|fingerprint| {
            let binding = ContinuationBinding::new(
                upstream.id(),
                upstream_model.clone(),
                connection.clone(),
                thinking_level.unwrap_or_default(),
                fingerprint,
            );
            self.continuations
                .take_if_current(upstream.as_ref(), handle.root_branch, &binding)
                .map(|cached| (cached, binding.context_fingerprint))
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
            Some(&profile),
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
            Some(self.provider_lifecycle_context()),
            Some(self.child_provider_resolver(service)),
        );
        summary_activity.settle();
        let result = match result {
            Ok(result) => result,
            Err(failure) => {
                self.continuations.discard(handle.root_branch);
                tracing::warn!(
                    operation_id = %handle.operation,
                    session_id = %handle.session,
                    branch_id = %handle.root_branch,
                    error = %failure.failure.error,
                    "agent root branch execution failed"
                );
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
        let outcome = if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            ContinuationOutcome::Cancelled
        } else {
            match (
                continuation,
                super::upstream_execution::completed_root_context_fingerprint(
                    service,
                    &runtime_context,
                    session,
                    handle.root_branch,
                    "",
                    selection.as_ref(),
                    &profile,
                ),
            ) {
                // Cancellation is rechecked after the recompile so a turn
                // cancelled while it settled retains nothing.
                (Some(extracted), Ok(fingerprint))
                    if !cancelled.load(std::sync::atomic::Ordering::Acquire) =>
                {
                    ContinuationOutcome::Completed(Box::new(CompletedContinuation {
                        binding: ContinuationBinding::new(
                            upstream.id(),
                            upstream_model,
                            connection,
                            thinking_level.unwrap_or_default(),
                            fingerprint,
                        ),
                        handle: extracted,
                    }))
                }
                (Some(_), Ok(_)) => ContinuationOutcome::Cancelled,
                _ => ContinuationOutcome::NoHandle,
            }
        };
        self.continuations
            .settle(upstream.as_ref(), handle.root_branch, outcome);
        Ok(completed)
    }
}

#[path = "runner/provider_packages.rs"]
mod provider_packages;
