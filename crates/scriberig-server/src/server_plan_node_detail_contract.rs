//! Defines the server contract for bounded plan-node gate/evidence reads.

macro_rules! server_plan_node_detail_contract {
    () => {
        /// Reads a bounded plan-node attempt page at an exact snapshot.
        ///
        /// - `_context`: Trusted request context.
        /// - `_request`: Authorized session, node, snapshot, and page range.
        ///
        /// # Returns
        /// Exact semantic attempt records.
        ///
        /// # Errors
        /// Returns authorization, history, or persistence failures.
        fn read_plan_attempt_history(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::PlanAttemptHistoryRequest,
        ) -> Result<Vec<scriberig_session::PlanSemanticAttempt>, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "plan attempt history read".to_owned(),
            })
        }

        /// Reads bounded per-node gate and evidence at an exact snapshot.
        ///
        /// - `_context`: Trusted request context.
        /// - `_request`: Authorized session, node, snapshot, and page range.
        ///
        /// # Returns
        /// Exact semantic gate and evidence records.
        ///
        /// # Errors
        /// Returns authorization, history, or persistence failures.
        fn read_plan_node_details(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::PlanNodeDetailsRequest,
        ) -> Result<scriberig_session::PlanSemanticNodeDetails, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "plan node detail read".to_owned(),
            })
        }
    };
}
