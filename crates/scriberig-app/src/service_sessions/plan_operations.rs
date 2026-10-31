//! Generates the application session plan-operation trait methods.

macro_rules! application_plan_operation_methods {
    () => {
        /// Creates a durable plan through the application plan service.
        ///
        /// - `context`: Authorized request context.
        /// - `request`: Goal, exact anchor, nodes, and dependencies.
        ///
        /// The authoritative persisted plan.
        ///
        /// Returns plan validation, authorization, or persistence failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn create_plan(
            &self,
            context: &ApplicationContext,
            request: &crate::PlanCreateRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            ApplicationService::create_plan(self, context, request)
        }

        /// Applies a replan proposal through the shared application plan service.
        ///
        /// - `context`: Authorized request context.
        /// - `request`: Complete plan revision proposal.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns plan validation, conflict, authorization, or persistence errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn replan(
            &self,
            context: &ApplicationContext,
            request: &crate::PlanReplanRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            ApplicationService::replan(self, context, request)
        }

        /// Applies one typed semantic plan mutation through the shared service.
        ///
        /// - `context`: Authorized request context.
        /// - `request`: Plan identity, expected revision, and typed mutation.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns validation, conflict, authorization, or persistence errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn mutate_plan(
            &self,
            context: &ApplicationContext,
            request: &crate::PlanMutationRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            ApplicationService::apply_plan_mutation(self, context, request)
        }

        /// Applies one lifecycle action through the shared application service.
        ///
        /// - `context`: Authorized request context.
        /// - `request`: Plan and expected revision.
        /// - `action`: Requested transition.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns plan transition, revision conflict, authorization, or persistence errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn plan_lifecycle(
            &self,
            context: &ApplicationContext,
            request: &crate::PlanLifecycleRequest,
            action: crate::PlanLifecycleAction,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            match action {
                crate::PlanLifecycleAction::Start => {
                    ApplicationService::start_plan(self, context, request)
                }
                crate::PlanLifecycleAction::Pause => {
                    ApplicationService::pause_plan(self, context, request)
                }
                crate::PlanLifecycleAction::Resume => {
                    ApplicationService::resume_plan(self, context, request)
                }
                crate::PlanLifecycleAction::Stop => {
                    ApplicationService::stop_plan(self, context, request)
                }
                crate::PlanLifecycleAction::Complete => {
                    ApplicationService::complete_plan(self, context, request)
                }
            }
        }

        /// Writes one validated node assignment through the application service.
        ///
        /// - `context`: Authorized request context.
        /// - `request`: Plan, node, revision, and credential-safe assignment.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns plan validation, authorization, or persistence failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_plan_node_assignment(
            &self,
            context: &ApplicationContext,
            request: &crate::PlanAssignmentRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            ApplicationService::set_plan_node_assignment(self, context, request)
        }
    };
}
