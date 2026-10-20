//! Contains the agent/session portion of the TUI service contract.

macro_rules! tui_agent_methods {
    () => {
        /// Starts a new conversation using application allocated identities.
        ///
        /// - `_context`: Application context for the request.
        /// - `_prompt`: User-authored first instruction.
        /// - `_selection`: Exact semantic scope for the new session.
        ///
        /// Authoritative operation, session, and root branch handles.
        ///
        /// Returns unsupported-operation when the adapter cannot start sessions.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn start_new_session_handle(
            &self,
            _context: &ApplicationContext,
            _prompt: &str,
            _selection: Option<scriberig_app::AgentSelection>,
        ) -> Result<AgentHandle, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "new session agent start".to_owned(),
            })
        }

        /// Continues an existing session from one exact durable anchor.
        ///
        /// - `_context`: Application context for the request.
        /// - `_session`: Owning session identity.
        /// - `_anchor`: Exact historical replyable location.
        /// - `_prompt`: User-authored continuation instruction.
        ///
        /// Authoritative operation, session, and continuation branch handles.
        ///
        /// Returns unsupported-operation when exact continuation is unavailable.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn continue_agent_at_handle(
            &self,
            _context: &ApplicationContext,
            _session: EntityId,
            _anchor: scriberig_session::ContinuationAnchor,
            _prompt: &str,
        ) -> Result<AgentHandle, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "exact agent continuation".to_owned(),
            })
        }

        /// Reads one branch subtree without requiring unrelated child histories.
        ///
        /// - `context`: Application context for the request.
        /// - `session`: Stable session identity.
        /// - `branch`: Branch whose activity should be loaded.
        ///
        /// The selected branch and nested descendants.
        ///
        /// Returns the underlying session read failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn read_session_branch(
            &self,
            context: &ApplicationContext,
            session: EntityId,
            branch: EntityId,
        ) -> Result<Session, ApplicationError> {
            Ok(select_branch_subtree(
                self.read_session(context, session)?,
                branch,
            ))
        }

        /// Submits agent text to a public session/branch operation.
        ///
        /// - `_context`: Application context for the request.
        /// - `_session`: Session identity receiving the prompt.
        /// - `_branch`: Optional branch identity.
        /// - `_prompt`: Agent prompt to submit.
        ///
        /// The accepted operation identity.
        ///
        /// Returns an unsupported-operation error by default.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn submit_agent_command(
            &self,
            _context: &ApplicationContext,
            _session: EntityId,
            _branch: Option<EntityId>,
            _prompt: &str,
        ) -> Result<scriberig_core::OperationId, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "agent command submission".to_owned(),
            })
        }

        /// Starts a shared agent operation at the session root.
        ///
        /// - `context`: Application context for the operation.
        /// - `session`: Session receiving the new root branch.
        /// - `prompt`: User-authored agent instruction.
        ///
        /// Stable accepted operation identity.
        ///
        /// Returns the service's structured agent submission error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn start_agent(
            &self,
            context: &ApplicationContext,
            session: EntityId,
            prompt: &str,
        ) -> Result<scriberig_core::OperationId, ApplicationError> {
            self.submit_agent_command(context, session, None, prompt)
        }

        /// Continues a shared agent operation on an existing branch.
        ///
        /// - `context`: Application context for the operation.
        /// - `session`: Session containing the branch.
        /// - `branch`: Existing branch receiving the follow-up.
        /// - `prompt`: User-authored follow-up instruction.
        ///
        /// Stable accepted operation identity.
        ///
        /// Returns the service's structured agent submission error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn continue_agent(
            &self,
            context: &ApplicationContext,
            session: EntityId,
            branch: EntityId,
            prompt: &str,
        ) -> Result<scriberig_core::OperationId, ApplicationError> {
            self.submit_agent_command(context, session, Some(branch), prompt)
        }

        /// Submits agent work and preserves stable handles available at acceptance.
        ///
        /// - `context`: Application context for the operation.
        /// - `session`: Session receiving the prompt.
        /// - `branch`: Optional continuation branch.
        /// - `prompt`: User-authored instruction.
        ///
        /// Stable operation, session, and known branch handles.
        ///
        /// Returns the service's structured agent submission error.
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
        ) -> Result<AgentHandle, ApplicationError> {
            let operation = branch.map_or_else(
                || self.start_agent(context, session, prompt),
                |branch| self.continue_agent(context, session, branch, prompt),
            )?;
            Ok(AgentHandle {
                operation,
                session,
                branch,
                workspace: None,
                failure: None,
            })
        }

        /// Runs a semantically scoped project agent through the shared candidate path.
        ///
        /// - `_context`: Trusted application context.
        /// - `_entity`: Selected semantic entity.
        /// - `_snapshot`: Exact candidate base snapshot.
        /// - `_prompt`: User instruction.
        ///
        /// The normal candidate review operation result.
        ///
        /// Returns an unsupported-operation error when no project agent is composed.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn project_agent(
            &self,
            _context: &ApplicationContext,
            _entity: EntityId,
            _snapshot: SnapshotId,
            _prompt: &str,
        ) -> Result<ApplicationResult, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "project agent".to_owned(),
            })
        }
    };
}
