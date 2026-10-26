//! Defines borrowed service delegation for legacy execution recovery.

macro_rules! server_borrowed_recovery_methods {
    () => {
        /// Delegates preserved legacy execution record reads to the borrowed service.
        ///
        /// - `context`: Trusted caller context.
        /// - `session`: Stable session identity.
        ///
        /// Sanitized recovery records.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns the delegated service error.
        fn legacy_execution_records(
            &self,
            context: &ApplicationContext,
            session: &str,
        ) -> Result<Vec<scriberig_protocol::LegacyExecutionRecordView>, ApplicationError> {
            (**self).legacy_execution_records(context, session)
        }

        /// Delegates explicit legacy selection mapping to the borrowed service.
        ///
        /// - `context`: Trusted caller context.
        /// - `request`: Exact recovery mapping request.
        ///
        /// Updated session execution selection.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns the delegated service error.
        fn map_legacy_execution_selection(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::LegacyExecutionMappingRequest,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            (**self).map_legacy_execution_selection(context, request)
        }

        /// Delegates current plan-assignment recovery to the borrowed service.
        ///
        /// - `context`: Trusted session and plan mutation context.
        /// - `request`: Exact legacy path, plan revision, and selected model.
        ///
        /// Updated protocol plan projection.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns the delegated service error.
        fn map_legacy_plan_assignment(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::LegacyPlanAssignmentMappingRequest,
        ) -> Result<scriberig_protocol::SessionPlan, ApplicationError> {
            (**self).map_legacy_plan_assignment(context, request)
        }
    };
}
