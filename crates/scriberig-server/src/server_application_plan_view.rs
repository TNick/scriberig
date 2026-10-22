//! Defines the application service compact plan view adapter method.

macro_rules! shared_plan_view_method {
    () => {
        /// Reads a compact plan graph from the shared semantic plan projection.
        ///
        /// # Parameters
        /// - `context`: Trusted session-read capability context.
        /// - `request`: Session, plan, and optional exact snapshot.
        ///
        /// # Returns
        /// Compact plan and node views with exact semantic revisions.
        ///
        /// # Errors
        /// Returns authorization, lookup, identity, or projection errors.
        fn read_plan_view(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::SessionPlanViewRequest,
        ) -> Result<scriberig_protocol::SessionPlanGraphView, ApplicationError> {
            super::server_plan_view::read(self, context, request)
        }
    };
}
