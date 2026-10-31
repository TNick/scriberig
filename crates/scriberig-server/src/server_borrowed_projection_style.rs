//! Supplies borrowed-service delegation for projection style operations.

/// Delegates projection style operations through a borrowed server service.
macro_rules! server_borrowed_projection_style_methods {
    () => {
        /// Delegates projection style schema reads.
        ///
        /// - `self`: Borrowed application service.
        /// - `context`: Trusted request context.
        /// - `projection_kind`: Stable projection kind name.
        ///
        /// The adapter schema when declared.
        ///
        /// # Returns
        /// The delegated schema response.
        ///
        /// # Errors
        /// Returns the delegated application error.
        fn projection_style_schema(
            &self,
            context: &ApplicationContext,
            projection_kind: &str,
        ) -> Result<Option<scriberig_protocol::ProjectionStyleSchemaResponse>, ApplicationError> {
            (**self).projection_style_schema(context, projection_kind)
        }

        /// Delegates purpose-aware style resolution.
        ///
        /// - `self`: Borrowed application service.
        /// - `context`: Trusted request context.
        /// - `request`: Exact style scopes and purpose.
        ///
        /// Effective style, provenance, and fidelity.
        ///
        /// # Returns
        /// The delegated resolution response.
        ///
        /// # Errors
        /// Returns the delegated application error.
        fn resolve_projection_style(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::ProjectionStyleResolveRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleResolveResponse, ApplicationError> {
            (**self).resolve_projection_style(context, request)
        }

        /// Delegates style setting mutations.
        ///
        /// - `self`: Borrowed application service.
        /// - `context`: Trusted request context.
        /// - `request`: Explicit setting scope and optional style value.
        ///
        /// Authoritative setting result.
        ///
        /// # Returns
        /// The delegated setting response.
        ///
        /// # Errors
        /// Returns the delegated application error.
        fn set_projection_style(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::ProjectionStyleSettingRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleSettingResponse, ApplicationError> {
            (**self).set_projection_style(context, request)
        }
    };
}
