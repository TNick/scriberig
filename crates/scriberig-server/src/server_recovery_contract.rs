//! Defines optional legacy recovery operations on the server service boundary.

macro_rules! server_recovery_contract_methods {
    () => {
        /// Changes the upstream provider/model used by future invocations.
        ///
        /// - `_context`: Trusted caller context.
        /// - `_session`: Stable session identity.
        /// - `_expected_revision`: Selection revision observed by the caller.
        /// - `_provider_id`: Exact upstream provider identity.
        /// - `_model_id`: Provider-scoped upstream model identity.
        ///
        /// Updated session execution selection.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_model(
            &self,
            _context: &ApplicationContext,
            _session: &str,
            _expected_revision: u64,
            _provider_id: &str,
            _model_id: &str,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "session model selection".to_owned(),
            })
        }

        /// Changes the thinking level used by future session invocations.
        ///
        /// - `_context`: Trusted caller context.
        /// - `_session`: Stable session identity.
        /// - `_expected_revision`: Selection revision observed by the caller.
        /// - `_thinking_level`: Advertised thinking identity.
        ///
        /// Updated session execution selection.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_thinking_level(
            &self,
            _context: &ApplicationContext,
            _session: &str,
            _expected_revision: u64,
            _thinking_level: &str,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "session thinking selection".to_owned(),
            })
        }

        /// Reads preserved legacy execution records for explicit recovery.
        ///
        /// - `_context`: Trusted caller context.
        /// - `_session`: Stable session identity.
        ///
        /// Sanitized records retained by the importer.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when authorization or storage fails.
        fn legacy_execution_records(
            &self,
            _context: &ApplicationContext,
            _session: &str,
        ) -> Result<Vec<scriberig_protocol::LegacyExecutionRecordView>, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "legacy execution recovery".to_owned(),
            })
        }

        /// Maps one preserved legacy session selection to an active upstream model.
        ///
        /// - `_context`: Trusted caller context.
        /// - `_request`: Exact record path, observed revision, and explicit selection.
        ///
        /// Updated session execution selection.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when validation, authorization, or persistence fails.
        fn map_legacy_execution_selection(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::LegacyExecutionMappingRequest,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "legacy execution recovery".to_owned(),
            })
        }

        /// Maps one current legacy plan assignment to active upstream IDs.
        ///
        /// - `_context`: Trusted session and plan mutation context.
        /// - `_request`: Exact recovery path, plan revision, and current model.
        ///
        /// Updated protocol plan projection.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when authorization, validation, or persistence fails.
        fn map_legacy_plan_assignment(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::LegacyPlanAssignmentMappingRequest,
        ) -> Result<scriberig_protocol::SessionPlan, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "legacy plan assignment recovery".to_owned(),
            })
        }
    };
}
