//! Implements shared query, list, and find methods for the server façade.

macro_rules! server_application_query_methods {
    () => {
        /// Executes a structured query through the shared application backend.
        /// - `context`: Argument named `context`.
        /// - `request`: Argument named `request`.
        ///
        ///
        /// Returns an error when the operation cannot complete.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn query(
            &self,
            context: &ApplicationContext,
            request: &QuerySpec,
        ) -> Result<QueryResult, ApplicationError> {
            match self.execute(
                context,
                &scriberig_app::ApplicationRequest::Query(request.clone()),
            )? {
                scriberig_app::ApplicationResult::Query(result) => Ok(result),
                _ => Err(ApplicationError::Internal {
                    message: "query backend returned a different result kind".to_owned(),
                }),
            }
        }

        /// Executes a bounded collection listing through the shared application.
        ///
        /// - `context`: Trusted request context.
        /// - `request`: Snapshot-bound list request.
        ///
        /// The application-owned list result.
        ///
        /// Returns the shared application failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn list(
            &self,
            context: &ApplicationContext,
            request: &ListRequest,
        ) -> Result<ListResult, ApplicationError> {
            match self.execute(
                context,
                &scriberig_app::ApplicationRequest::List(request.clone()),
            )? {
                scriberig_app::ApplicationResult::List(result) => Ok(result),
                _ => Err(ApplicationError::Internal {
                    message: "list backend returned a different result kind".to_owned(),
                }),
            }
        }

        /// Executes semantic discovery through the shared application.
        ///
        /// - `context`: Trusted request context.
        /// - `request`: Snapshot-scoped discovery request.
        ///
        /// The application-owned discovery result.
        ///
        /// Returns the shared application failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn find(
            &self,
            context: &ApplicationContext,
            request: &FindRequest,
        ) -> Result<FindResult, ApplicationError> {
            match self.execute(
                context,
                &scriberig_app::ApplicationRequest::Find(request.clone()),
            )? {
                scriberig_app::ApplicationResult::Find(result) => Ok(result),
                _ => Err(ApplicationError::Internal {
                    message: "find backend returned a different result kind".to_owned(),
                }),
            }
        }
    };
}
