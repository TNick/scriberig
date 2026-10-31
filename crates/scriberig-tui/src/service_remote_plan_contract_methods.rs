//! Contains remote plan and execution selection contract methods.

macro_rules! remote_plan_contract_methods {
    () => {
        /// Creates a durable plan through the shared remote protocol operation.
        ///
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `request`: Structured goal, exact anchor, nodes, and dependencies.
        ///
        /// The authoritative persisted plan.
        ///
        /// Returns a sanitized remote application error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn create_plan(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::PlanCreateRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            plan_operations::create(self, request)
        }

        /// Applies one plan revision through the remote protocol operation.
        ///
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `request`: Complete plan revision proposal.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns a sanitized remote application error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn replan(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::PlanReplanRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            plan_operations::replan(self, request)
        }

        /// Applies one lifecycle action through the shared remote operation.
        ///
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `request`: Plan identity and observed revision.
        /// - `action`: Requested lifecycle transition.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns a sanitized remote application error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn plan_lifecycle(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::PlanLifecycleRequest,
            action: scriberig_app::PlanLifecycleAction,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            plan_operations::lifecycle(self, request, action)
        }

        /// Applies a typed semantic plan mutation through the shared remote operation.
        ///
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `request`: Plan identity, observed revision, and typed mutation.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns a sanitized remote application error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn mutate_plan(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::PlanMutationRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            plan_operations::mutate(self, request)
        }

        /// Changes one node assignment through the shared remote operation.
        ///
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `request`: Plan, node, observed revision, and assignment.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns a sanitized remote application error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_plan_node_assignment(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::PlanAssignmentRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            plan_operations::assign(self, request)
        }

        /// Reads bounded safe result detail through the protocol client.
        /// - `_context`: Authorized application context.
        /// - `session`: Session containing the result.
        /// - `branch`: Branch containing the result.
        /// - `result`: Stable tool-result identity.
        ///
        /// Bounded allowlisted result detail.
        ///
        /// Returns a sanitized remote protocol error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn tool_result_detail(
            &self,
            _context: &ApplicationContext,
            session: EntityId,
            branch: EntityId,
            result: EntityId,
        ) -> Result<scriberig_protocol::ToolResultDetailResponse, ApplicationError> {
            service_remote_models::tool_result_detail(self, session, branch, result)
        }

        /// Reads credential-safe model descriptors from the public client.
        ///
        /// - `_context`: Authorized application context.
        ///
        /// Safe model and profile descriptors.
        ///
        /// Returns a sanitized remote protocol error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn model_library(
            &self,
            _context: &ApplicationContext,
        ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
            service_remote_models::model_library(self)
        }

        /// Updates future model selection through the revision-checked protocol.
        ///
        /// - `_context`: Authorized application context.
        /// - `request`: Session and observed selection revision.
        ///
        /// The authoritative updated execution selection.
        ///
        /// Returns a sanitized remote protocol error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_model(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::SessionModelSelectionRequest,
        ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
            service_remote_models::set_session_model(self, request)
        }

        /// Updates future thinking selection through the revision-checked protocol.
        ///
        /// - `_context`: Authorized application context.
        /// - `request`: Session and observed selection revision.
        ///
        /// The authoritative updated execution selection.
        ///
        /// Returns a sanitized remote protocol error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_thinking_level(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::SessionThinkingSelectionRequest,
        ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
            service_remote_models::set_session_thinking_level(self, request)
        }
    };
}
