//! Defines shared model and thinking selection adapter methods.

macro_rules! server_execution_selection_methods {
    () => {
        /// Selects an upstream provider/model pair for future invocations.
        ///
        /// - `context`: Trusted capability context.
        /// - `session`: Stable session identity.
        /// - `expected_revision`: Selection revision observed by the caller.
        /// - `provider_id`: Exact upstream provider identity.
        /// - `model_id`: Provider-scoped upstream model identity.
        ///
        /// Updated session execution selection.
        ///
        /// Returns authorization, validation, conflict, or persistence failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_model(
            &self,
            context: &ApplicationContext,
            session: &str,
            expected_revision: u64,
            provider_id: &str,
            model_id: &str,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            server_agent_console::set_session_model(
                self,
                context,
                session,
                expected_revision,
                provider_id,
                model_id,
            )
        }

        /// Selects a thinking level for future session invocations.
        ///
        /// - `context`: Trusted capability context.
        /// - `session`: Stable session identity.
        /// - `expected_revision`: Selection revision observed by the caller.
        /// - `thinking_level`: Advertised thinking capability identity.
        ///
        /// Updated session execution selection.
        ///
        /// Returns authorization, validation, conflict, or persistence failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_thinking_level(
            &self,
            context: &ApplicationContext,
            session: &str,
            expected_revision: u64,
            thinking_level: &str,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            server_agent_console::set_session_thinking_level(
                self,
                context,
                session,
                expected_revision,
                thinking_level,
            )
        }

        /// Reads preserved legacy execution records for explicit recovery.
        ///
        /// - `context`: Trusted caller context.
        /// - `session`: Stable session identity.
        ///
        /// Sanitized legacy records retained by the importer.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when authorization or storage fails.
        fn legacy_execution_records(
            &self,
            context: &ApplicationContext,
            session: &str,
        ) -> Result<Vec<scriberig_protocol::LegacyExecutionRecordView>, ApplicationError> {
            server_agent_console::legacy_execution_records(self, context, session)
        }

        /// Maps one preserved session execution record to an active upstream model.
        ///
        /// - `context`: Trusted caller context.
        /// - `request`: Exact record path, observed revision, and explicit selection.
        ///
        /// Updated session execution selection.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when validation, authorization, or storage fails.
        fn map_legacy_execution_selection(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::LegacyExecutionMappingRequest,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            server_agent_console::map_legacy_execution_selection(self, context, request)
        }

        /// Maps one current legacy plan assignment to active upstream IDs.
        ///
        /// - `context`: Trusted session and plan mutation context.
        /// - `request`: Exact legacy path, plan revision, and selected model.
        ///
        /// Updated protocol plan projection.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when authorization, validation, or persistence fails.
        fn map_legacy_plan_assignment(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::LegacyPlanAssignmentMappingRequest,
        ) -> Result<scriberig_protocol::SessionPlan, ApplicationError> {
            server_agent_console::map_legacy_plan_assignment(self, context, request)
        }
    };
}
