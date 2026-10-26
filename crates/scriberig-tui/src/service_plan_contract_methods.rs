//! Contains the plan, result detail, and model selection service contract.

macro_rules! tui_plan_methods {
    () => {
        /// Creates a durable plan through the shared application operation.
        ///
        /// - `_context`: Authorized application request context.
        /// - `_request`: Structured goal, anchor, nodes, and dependencies.
        ///
        /// The persisted authoritative plan.
        ///
        /// Returns unsupported-operation or structured plan errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn create_plan(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::PlanCreateRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "TUI plan creation".to_owned(),
            })
        }

        /// Applies a structured replan through the shared plan service.
        ///
        /// - `_context`: Authorized application request context.
        /// - `_request`: Complete plan revision proposal.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns unsupported-operation or structured plan errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn replan(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::PlanReplanRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "TUI plan replan".to_owned(),
            })
        }

        /// Applies one optimistic plan lifecycle operation.
        ///
        /// - `_context`: Authorized application request context.
        /// - `_request`: Plan identity and expected revision.
        /// - `_action`: Requested lifecycle transition.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns unsupported-operation or structured plan errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn plan_lifecycle(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::PlanLifecycleRequest,
            _action: scriberig_app::PlanLifecycleAction,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "TUI plan lifecycle".to_owned(),
            })
        }

        /// Applies one typed semantic plan mutation through the shared operation.
        ///
        /// - `_context`: Authorized application request context.
        /// - `_request`: Plan identity, observed revision, and typed mutation.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns unsupported-operation or structured plan errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn mutate_plan(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::PlanMutationRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "TUI semantic plan mutation".to_owned(),
            })
        }

        /// Changes a plan node's credential-safe model assignment.
        ///
        /// - `_context`: Authorized application request context.
        /// - `_request`: Plan, node, observed revision, and assignment.
        ///
        /// The authoritative updated plan.
        ///
        /// Returns unsupported-operation or structured plan errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_plan_node_assignment(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::PlanAssignmentRequest,
        ) -> Result<scriberig_session::Plan, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "TUI plan assignment".to_owned(),
            })
        }

        /// Reads bounded safe detail for one tool result.
        /// - `_context`: Authorized request context.
        /// - `_session`: Session containing the result.
        /// - `_branch`: Branch containing the result.
        /// - `_result`: Stable tool-result identity.
        ///   Bounded allowlisted result detail.
        ///   Returns unsupported-operation when unavailable.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn tool_result_detail(
            &self,
            _context: &ApplicationContext,
            _session: EntityId,
            _branch: EntityId,
            _result: EntityId,
        ) -> Result<scriberig_protocol::ToolResultDetailResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "tool result detail".to_owned(),
            })
        }

        /// Reads the credential-safe model library for interactive selection.
        ///
        /// - `_context`: Application context for the request.
        ///
        /// Safe model and profile descriptors.
        ///
        /// Returns an unsupported-operation error by default.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn model_library(
            &self,
            _context: &ApplicationContext,
        ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "model library".to_owned(),
            })
        }

        /// Updates a session's future model selection using its observed revision.
        ///
        /// - `_context`: Application context for the mutation.
        /// - `_request`: Session, expected revision, and selected profile.
        ///
        /// The authoritative updated execution selection.
        ///
        /// Returns an unsupported-operation error by default.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_model(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::SessionModelSelectionRequest,
        ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "session model selection".to_owned(),
            })
        }

        /// Updates a session's future thinking selection using its revision.
        ///
        /// - `_context`: Application context for the mutation.
        /// - `_request`: Session, expected revision, and thinking level.
        ///
        /// The authoritative updated execution selection.
        ///
        /// Returns an unsupported-operation error by default.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_thinking_level(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::SessionThinkingSelectionRequest,
        ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "session thinking selection".to_owned(),
            })
        }
    };
}
