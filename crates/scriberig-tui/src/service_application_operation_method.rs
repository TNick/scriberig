//! Defines the generic application-operation method for TUI service adapters.

macro_rules! tui_application_operation_methods {
    () => {
        /// Executes one shared application operation when the adapter supports it.
        ///
        /// - `_context`: Application context for the operation.
        /// - `_request`: Public operation request.
        ///
        /// The operation result when supported.
        ///
        /// Returns a structured error when the operation is unavailable or rejected.
        /// # Returns
        /// The application operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn execute_operation(
            &self,
            _context: &ApplicationContext,
            _request: &ApplicationRequest,
        ) -> Result<ApplicationResult, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "tui semantic operation".to_owned(),
            })
        }
    };
}
