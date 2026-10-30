//! Forwards generic query methods through a borrowed application service.

macro_rules! server_borrowed_query_methods {
    () => {
        /// Delegates semantic discovery to the borrowed application service.
        ///
        /// - `context`: Trusted request context.
        /// - `request`: Snapshot-scoped discovery request.
        ///
        /// The service's bounded discovery result.
        ///
        /// Returns the service's structured failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn find(
            &self,
            context: &ApplicationContext,
            request: &FindRequest,
        ) -> Result<FindResult, ApplicationError> {
            (**self).find(context, request)
        }

        /// Delegates bounded collection listing to the borrowed service.
        ///
        /// - `context`: Trusted request context.
        /// - `request`: Snapshot-bound collection request.
        ///
        /// The service's bounded collection page.
        ///
        /// Returns the service's structured failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn list(
            &self,
            context: &ApplicationContext,
            request: &ListRequest,
        ) -> Result<ListResult, ApplicationError> {
            (**self).list(context, request)
        }

        /// Delegates structured queries to the borrowed application service.
        ///
        /// - `context`: Trusted exact-snapshot context.
        /// - `request`: Typed query specification.
        ///
        /// # Returns
        /// The service's structured query result.
        ///
        /// # Errors
        /// Returns a structured failure when the query cannot be completed.
        fn query(
            &self,
            context: &ApplicationContext,
            request: &QuerySpec,
        ) -> Result<QueryResult, ApplicationError> {
            (**self).query(context, request)
        }

        /// Delegates declared commands to the borrowed application service.
        ///
        /// - `context`: Trusted request context.
        /// - `kind`: Stable command family.
        /// - `payload`: Typed command payload.
        ///
        /// # Returns
        /// The accepted or completed shared operation.
        ///
        /// # Errors
        /// Returns a structured failure when the command cannot complete.
        fn command(
            &self,
            context: &ApplicationContext,
            kind: &str,
            payload: &serde_json::Value,
        ) -> Result<Operation, ApplicationError> {
            (**self).command(context, kind, payload)
        }
    };
}
