//! Contains the agent/session portion of the TUI service contract.

macro_rules! tui_agent_methods {
    () => {
        /// Reads one branch subtree without requiring unrelated child histories.
        ///
        /// # Parameters
        /// - `context`: Application context for the request.
        /// - `session`: Stable session identity.
        /// - `branch`: Branch whose activity should be loaded.
        ///
        /// # Returns
        /// The selected branch and nested descendants.
        ///
        /// # Errors
        /// Returns the underlying session read failure.
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
        /// # Parameters
        /// - `_context`: Application context for the request.
        /// - `_session`: Session identity receiving the prompt.
        /// - `_branch`: Optional branch identity.
        /// - `_prompt`: Agent prompt to submit.
        ///
        /// # Returns
        /// The accepted operation identity.
        ///
        /// # Errors
        /// Returns an unsupported-operation error by default.
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
        /// # Parameters
        /// - `context`: Application context for the operation.
        /// - `session`: Session receiving the new root branch.
        /// - `prompt`: User-authored agent instruction.
        ///
        /// # Returns
        /// Stable accepted operation identity.
        ///
        /// # Errors
        /// Returns the service's structured agent submission error.
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
        /// # Parameters
        /// - `context`: Application context for the operation.
        /// - `session`: Session containing the branch.
        /// - `branch`: Existing branch receiving the follow-up.
        /// - `prompt`: User-authored follow-up instruction.
        ///
        /// # Returns
        /// Stable accepted operation identity.
        ///
        /// # Errors
        /// Returns the service's structured agent submission error.
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
        /// # Parameters
        /// - `context`: Application context for the operation.
        /// - `session`: Session receiving the prompt.
        /// - `branch`: Optional continuation branch.
        /// - `prompt`: User-authored instruction.
        ///
        /// # Returns
        /// Stable operation, session, and known branch handles.
        ///
        /// # Errors
        /// Returns the service's structured agent submission error.
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
            })
        }

        /// Runs a semantically scoped project agent through the shared candidate path.
        ///
        /// # Parameters
        /// - `_context`: Trusted application context.
        /// - `_entity`: Selected semantic entity.
        /// - `_snapshot`: Exact candidate base snapshot.
        /// - `_prompt`: User instruction.
        ///
        /// # Returns
        /// The normal candidate review operation result.
        ///
        /// # Errors
        /// Returns an unsupported-operation error when no project agent is composed.
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
