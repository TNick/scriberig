//! Supplies projection style adapters for the shared application service.

/// Delegates wire style operations to the shared application projection-style backend.
macro_rules! server_application_projection_style_methods {
    () => {
        /// Reads one domain-owned style schema through the application service.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted request context.
        /// - `projection_kind`: Stable projection kind name.
        ///
        /// The adapter schema when one is declared.
        ///
        /// # Returns
        /// The optional schema response.
        ///
        /// # Errors
        /// Returns application errors from schema discovery.
        fn projection_style_schema(
            &self,
            context: &ApplicationContext,
            projection_kind: &str,
        ) -> Result<Option<scriberig_protocol::ProjectionStyleSchemaResponse>, ApplicationError> {
            crate::server_projection_style::schema(self, context, projection_kind)
        }

        /// Resolves one effective style through application-owned precedence rules.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted request context and principal.
        /// - `request`: Purpose and scope requested by the caller.
        ///
        /// Effective style, provenance, and exact fidelity.
        ///
        /// # Returns
        /// The style resolution response.
        ///
        /// # Errors
        /// Returns application errors for invalid or unavailable style inputs.
        fn resolve_projection_style(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::ProjectionStyleResolveRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleResolveResponse, ApplicationError> {
            crate::server_projection_style::resolve(self, context, request)
        }

        /// Sets or clears a style setting through application-owned validation and storage.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted request context and principal.
        /// - `request`: Explicit scope, kind, expected revision, and style bytes.
        ///
        /// The authoritative setting state.
        ///
        /// # Returns
        /// The current stored value and revision.
        ///
        /// # Errors
        /// Returns application authorization, validation, conflict, or storage errors.
        fn set_projection_style(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::ProjectionStyleSettingRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleSettingResponse, ApplicationError> {
            crate::server_projection_style::set(self, context, request)
        }
    };
}
