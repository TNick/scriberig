//! Forwards compact session reads through borrowed application services.

macro_rules! shared_borrowed_session_read_method {
    () => {
        /// Delegates compact session reads to the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context.
        /// - `session`: Stable session identity.
        ///
        /// # Returns
        /// Compact session tree response.
        ///
        /// # Errors
        /// Returns the service's structured failure.
        fn read_session(
            &self,
            context: &ApplicationContext,
            session: &str,
        ) -> Result<scriberig_protocol::SessionReadResponse, ApplicationError> {
            (**self).read_session(context, session)
        }
    };
}
