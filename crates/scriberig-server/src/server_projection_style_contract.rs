//! Declares the projection style operations on the shared server service.

/// Provides optional shared projection style capabilities to server services.
macro_rules! server_projection_style_contract {
    () => {
        /// Reads a domain-owned projection style schema.
        ///
        /// - `self`: Application service providing style semantics.
        /// - `_context`: Trusted request context.
        /// - `_projection_kind`: Stable projection kind name.
        ///
        /// The schema when an installed domain adapter declares one.
        ///
        /// # Returns
        /// The optional schema.
        ///
        /// # Errors
        /// Returns an application error when the style subsystem cannot answer.
        fn projection_style_schema(
            &self,
            _context: &scriberig_app::ApplicationContext,
            _projection_kind: &str,
        ) -> Result<Option<scriberig_protocol::ProjectionStyleSchemaResponse>, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "projection style schema".to_owned(),
            })
        }

        /// Resolves one purpose-aware style through shared application semantics.
        ///
        /// - `self`: Application service providing style semantics.
        /// - `_context`: Trusted request context; principal identity is host supplied.
        /// - `_request`: Exact projection kind, purpose, and scope.
        ///
        /// Effective style, source provenance, and fidelity.
        ///
        /// # Returns
        /// The resolved wire contract.
        ///
        /// # Errors
        /// Returns a structured application error for invalid scope or style input.
        fn resolve_projection_style(
            &self,
            _context: &scriberig_app::ApplicationContext,
            _request: &scriberig_protocol::ProjectionStyleResolveRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleResolveResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "projection style resolution".to_owned(),
            })
        }

        /// Sets or clears one style setting in an explicit configuration scope.
        ///
        /// - `self`: Application service providing style semantics.
        /// - `_context`: Trusted request context and caller identity.
        /// - `_request`: Setting scope, kind, revision, and optional style bytes.
        ///
        /// The authoritative stored setting state.
        ///
        /// # Returns
        /// The setting value and revision, or none after a clear.
        ///
        /// # Errors
        /// Returns permission, conflict, validation, or persistence errors.
        fn set_projection_style(
            &self,
            _context: &scriberig_app::ApplicationContext,
            _request: &scriberig_protocol::ProjectionStyleSettingRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleSettingResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "projection style setting".to_owned(),
            })
        }
    };
}
