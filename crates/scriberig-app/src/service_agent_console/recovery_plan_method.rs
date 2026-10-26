//! Defines the application façade for explicit legacy plan-assignment mapping.

macro_rules! service_legacy_plan_assignment_method {
    () => {
        /// Maps one current legacy plan assignment to an active upstream model.
        ///
        /// - `context`: Authorized session and plan mutation context.
        /// - `request`: Exact legacy path, observed plan revision, and current model.
        ///
        /// Updated durable plan with the matching recovery record removed.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns authorization, catalog, conflict, or storage failures.
        fn map_legacy_plan_assignment(
            &self,
            context: &ApplicationContext,
            request: &LegacyPlanAssignmentMappingRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            recovery::map_legacy_plan_assignment(self, context, request)
        }
    };
}
