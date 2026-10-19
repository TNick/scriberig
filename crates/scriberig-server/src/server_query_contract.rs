//! Declares default query operations for the server application contract.

macro_rules! server_query_contract {
    () => {
        /// Executes one structured query through the application boundary.
        ///
        /// - `_context`: Trusted request context.
        /// - `_request`: Structured query specification.
        ///
        /// # Returns
        /// The structured query result.
        ///
        /// # Errors
        /// Returns an error when query execution is unavailable.
        fn query(
            &self,
            _context: &ApplicationContext,
            _request: &QuerySpec,
        ) -> Result<QueryResult, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "query".to_owned(),
            })
        }

        /// Executes one bounded semantic collection listing.
        ///
        /// - `_context`: Trusted request context.
        /// - `_request`: Snapshot-bound collection request.
        ///
        /// A bounded collection page.
        ///
        /// Returns unsupported-operation when listing is unavailable.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn list(
            &self,
            _context: &ApplicationContext,
            _request: &ListRequest,
        ) -> Result<ListResult, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "list".to_owned(),
            })
        }

        /// Executes bounded semantic discovery within an exact scope.
        ///
        /// - `_context`: Trusted request context.
        /// - `_request`: Snapshot-scoped discovery request.
        ///
        /// A bounded discovery result.
        ///
        /// Returns unsupported-operation when discovery is unavailable.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn find(
            &self,
            _context: &ApplicationContext,
            _request: &FindRequest,
        ) -> Result<FindResult, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "find".to_owned(),
            })
        }

        /// Executes one declared application command and returns its lifecycle state.
        ///
        /// Returns an application error when command execution is unavailable.
        /// - `_context`: Argument named `_context`.
        /// - `_kind`: Argument named `_kind`.
        /// - `_payload`: Argument named `_payload`.
        ///
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn command(
            &self,
            _context: &ApplicationContext,
            _kind: &str,
            _payload: &serde_json::Value,
        ) -> Result<Operation, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "command".to_owned(),
            })
        }
    };
}
