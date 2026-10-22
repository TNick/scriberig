//! Defines compact plan view forwarding for borrowed application services.

macro_rules! shared_borrowed_plan_view_method {
    () => {
        /// Reads a compact plan graph through a borrowed application service.
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
            (**self).read_plan_view(context, request)
        }
    };
}

macro_rules! shared_borrowed_plan_detail_methods {
    () => {
        /// Delegates bounded plan-attempt reads to the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted session-read capability context.
        /// - `request`: Session, plan, node, and page selection.
        ///
        /// # Returns
        /// The safe bounded attempt page.
        ///
        /// # Errors
        /// Returns the delegated service failure.
        fn read_plan_attempts(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::SessionPlanAttemptsRequest,
        ) -> Result<scriberig_protocol::SessionPlanAttemptsResponse, ApplicationError> {
            (**self).read_plan_attempts(context, request)
        }

        /// Delegates bounded plan requirement reads to the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted session-read capability context.
        /// - `request`: Session, plan revision, and page selection.
        ///
        /// # Returns
        /// Bounded gate and evidence detail.
        ///
        /// # Errors
        /// Returns the delegated service failure.
        fn read_plan_details(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::SessionPlanDetailsRequest,
        ) -> Result<scriberig_protocol::SessionPlanDetailsResponse, ApplicationError> {
            (**self).read_plan_details(context, request)
        }
    };
}
