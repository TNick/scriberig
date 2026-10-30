//! Contains shared application plan mutation contract methods.

macro_rules! application_plan_mutation_methods {
    () => {
        /// Retries one failed node through the shared application service.
        ///
        /// - `context`: Caller authorization context.
        /// - `request`: Plan, node, and observed plan revision.
        ///
        /// The authoritative updated plan.
        /// # Returns
        /// The updated plan projection.
        /// # Errors
        /// Returns application validation, conflict, or persistence errors.
        fn retry_plan_node(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::PlanRetryRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            scriberig_app::SessionBackend::retry_plan_node(&self.service, context, request)
        }

        /// Adds the next work item through the shared application service.
        ///
        /// - `context`: Caller authorization context.
        /// - `request`: Plan revision and proposed work item.
        ///
        /// The authoritative updated plan.
        /// # Returns
        /// The updated plan projection.
        /// # Errors
        /// Returns application validation, conflict, or persistence errors.
        fn add_plan_next_step(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::PlanNextStepRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            scriberig_app::SessionBackend::add_plan_next_step(&self.service, context, request)
        }

        /// Decomposes one node through the shared application service.
        ///
        /// - `context`: Caller authorization context.
        /// - `request`: Parent node, children, and observed plan revision.
        ///
        /// The authoritative updated plan.
        /// # Returns
        /// The updated plan projection.
        /// # Errors
        /// Returns application validation, conflict, or persistence errors.
        fn decompose_plan_node(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::PlanDecomposeRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            scriberig_app::SessionBackend::decompose_plan_node(&self.service, context, request)
        }

        /// Applies a typed semantic mutation through the shared application service.
        ///
        /// - `context`: Caller authorization context used for authorization and provenance.
        /// - `request`: Exact plan revision and typed mutation.
        ///
        /// The authoritative updated plan.
        /// # Returns
        /// The updated plan projection.
        /// # Errors
        /// Returns application validation, conflict, or persistence errors.
        fn mutate_plan_semantics(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::PlanSemanticMutationRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            scriberig_app::SessionBackend::mutate_plan_semantics(&self.service, context, request)
        }
    };
}
