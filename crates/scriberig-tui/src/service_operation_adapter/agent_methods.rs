//! Implements model configuration and agent runtime adapter methods.

macro_rules! operation_agent_methods {
    () => {
        /// Reads upstream safe model snapshots through the shared application.
        ///
        /// - `context`: Authorized caller context.
        ///
        /// Upstream model display rows without provider credentials.
        ///
        /// Returns an authorization or model-library configuration error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn model_library(
            &self,
            context: &ApplicationContext,
        ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
            let models =
                scriberig_app::AgentConsoleBackend::agent_model_library(&self.service, context)?;
            let mut response = super::service_application::query::model_library_response(models);
            super::service_application::query::attach_provider_readiness(
                &mut response,
                self.agent_runner.provider_readiness(),
                self.service.model_catalog_generation(),
            );
            Ok(response)
        }

        /// Updates the selected session model through the shared application.
        ///
        /// - `context`: Authorized caller context.
        /// - `request`: Session and revision-bound model profile selection.
        ///
        /// The authoritative future execution selection.
        ///
        /// Returns an authorization, conflict, or model-profile error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_model(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::SessionModelSelectionRequest,
        ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
            scriberig_app::AgentConsoleBackend::set_session_model(&self.service, context, request)
        }

        /// Updates the selected session thinking level through the shared application.
        ///
        /// - `context`: Authorized caller context.
        /// - `request`: Session and revision-bound thinking selection.
        ///
        /// The authoritative future execution selection.
        ///
        /// Returns an authorization, conflict, or thinking-level error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_thinking_level(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::SessionThinkingSelectionRequest,
        ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
            scriberig_app::AgentConsoleBackend::set_session_thinking_level(
                &self.service,
                context,
                request,
            )
        }

        /// Applies a plan transition through the configured product runtime.
        ///
        /// - `self`: Embedded adapter with its injected runtime.
        /// - `context`: Trusted caller capabilities and scope.
        /// - `request`: Plan identity and observed revision.
        /// - `action`: Requested transition.
        ///
        /// The authoritative plan after the optional coordinator run.
        ///
        /// Returns a lifecycle or worker execution failure.
        /// # Returns
        /// The authoritative plan.
        /// # Errors
        /// Returns an error when the transition or execution fails.
        fn plan_lifecycle(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::PlanLifecycleRequest,
            action: scriberig_app::PlanLifecycleAction,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            self.agent_runner
                .plan_lifecycle(&self.service, context, request, action)
        }

        /// Starts a new conversation and runs its already accepted operation.
        ///
        /// - `context`: Trusted application context.
        /// - `prompt`: User-authored first instruction.
        /// - `selection`: Exact semantic scope for the initial model context.
        ///
        /// Stable handles after the composed runtime finishes the first turn.
        ///
        /// Returns a session acceptance or runtime execution failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn start_new_session_handle(
            &self,
            context: &ApplicationContext,
            prompt: &str,
            selection: Option<scriberig_app::AgentSelection>,
        ) -> Result<super::AgentHandle, ApplicationError> {
            let accepted = self.service.begin_new_session_agent(
                context,
                scriberig_app::NewSessionAgentRequest {
                    prompt: prompt.to_owned(),
                    selection: selection.clone(),
                },
            )?;
            Ok(accepted_agent::execute(
                self.agent_runner.as_ref(),
                &self.service,
                context,
                &accepted,
            ))
        }

        /// Continues an exact anchor and runs its accepted continuation operation.
        ///
        /// - `context`: Trusted application context.
        /// - `session`: Existing durable session.
        /// - `anchor`: Exact replyable continuation location.
        /// - `prompt`: User-authored continuation instruction.
        ///
        /// Stable operation, session, and child branch handles.
        ///
        /// Returns an anchor validation or runtime execution failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn continue_agent_at_handle(
            &self,
            context: &ApplicationContext,
            session: EntityId,
            anchor: scriberig_session::ContinuationAnchor,
            prompt: &str,
        ) -> Result<super::AgentHandle, ApplicationError> {
            let accepted = self.service.begin_agent_execution(
                context,
                &scriberig_app::AgentCommandRequest {
                    session,
                    branch: None,
                    anchor: Some(anchor),
                    prompt: prompt.to_owned(),
                    selection: None,
                },
            )?;
            Ok(accepted_agent::execute(
                self.agent_runner.as_ref(),
                &self.service,
                context,
                &accepted,
            ))
        }

        /// Submits local agent work through the runtime composed by the product.
        ///
        /// - `self`: Adapter with the injected shared runtime boundary.
        /// - `context`: Application context for the operation.
        /// - `session`: Stable session identity.
        /// - `branch`: Optional continuation branch.
        /// - `prompt`: User-authored instruction.
        ///
        /// Stable operation, session, and branch handles.
        ///
        /// Returns an application submission or session-read error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn submit_agent_handle(
            &self,
            context: &ApplicationContext,
            session: EntityId,
            branch: Option<EntityId>,
            prompt: &str,
        ) -> Result<super::AgentHandle, ApplicationError> {
            self.agent_runner
                .submit(&self.service, context, session, branch, prompt)
        }
    };
}
