//! Declares optional CAD operations available through the shared server boundary.

macro_rules! shared_cad_contract {
    () => {
        /// Proposes an exact-base CAD line geometry edit through shared authorization.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, snapshot, and scoped grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_entity`: Serialized stable CAD line identity.
        /// - `_start`: Replacement exact decimal start coordinate.
        /// - `_end`: Replacement exact decimal end coordinate.
        ///
        /// # Returns
        /// Durable candidate handle for shared validation and review.
        ///
        /// # Errors
        /// Returns unsupported operation unless CAD writes are implemented.
        fn propose_cad_line_edit(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _entity: &str,
            _start: scriberig_protocol::CadCoordinate,
            _end: scriberig_protocol::CadCoordinate,
        ) -> Result<scriberig_app::CadLineCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "cad.line.propose".into(),
            })
        }

        /// Proposes one exact-base CAD entity layer change through shared authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted caller context.
        /// - `_project`: Serialized stable project identity.
        /// - `_entity`: Serialized semantic CAD entity identity.
        /// - `_destination_layer`: Serialized semantic layer identity.
        ///
        /// # Returns
        /// Durable candidate handle for shared validation and review.
        ///
        /// # Errors
        /// Returns unsupported operation unless CAD layer changes are implemented.
        fn propose_cad_layer_change(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _entity: &str,
            _destination_layer: &str,
        ) -> Result<scriberig_app::CadLayerCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "cad.layer.propose".into(),
            })
        }

        /// Reads an exact-snapshot CAD drawing summary through shared Inspect authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_snapshot`: Serialized exact snapshot identity.
        /// - `_drawing`: Serialized semantic drawing identity.
        ///
        /// # Returns
        /// Typed drawing summary with semantic layers.
        ///
        /// # Errors
        /// Returns unsupported operation when this service has no CAD reader.
        fn inspect_cad_drawing(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _snapshot: &str,
            _drawing: &str,
        ) -> Result<scriberig_project::CadDrawingView, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "cad.drawing.inspect".into(),
            })
        }

        /// Reads one bounded exact-snapshot CAD entity page through Inspect authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_snapshot`: Serialized exact snapshot identity.
        /// - `_drawing`: Serialized semantic drawing identity.
        /// - `_offset`: Zero-based source-order offset.
        /// - `_limit`: Bounded maximum entity count.
        ///
        /// # Returns
        /// Typed semantic entity page.
        ///
        /// # Errors
        /// Returns unsupported operation when this service has no CAD reader.
        fn read_cad_entities(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _snapshot: &str,
            _drawing: &str,
            _offset: usize,
            _limit: usize,
        ) -> Result<scriberig_project::CadEntityPage, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "cad.entities.read".into(),
            })
        }

        /// Finds authored CAD annotation text through shared Inspect authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and grants.
        /// - `_request`: Exact-snapshot drawing, query, and bounded page parameters.
        ///
        /// # Returns
        /// Bounded text matches with stable semantic entity identities.
        ///
        /// # Errors
        /// Returns unsupported operation when this service has no CAD search.
        fn find_cad_text(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::CadTextSearchRequest,
        ) -> Result<scriberig_project::CadTextSearch, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "cad.text.find".into(),
            })
        }

        /// Reads CAD entities intersecting one exact-snapshot drawing-space envelope.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and grants.
        /// - `_request`: Validated semantic spatial query.
        ///
        /// # Returns
        /// Bounded semantic entities with extent completeness metadata.
        ///
        /// # Errors
        /// Returns unsupported operation unless CAD bounds queries are implemented.
        fn read_cad_bounds_query(
            &self,
            _context: &ApplicationContext,
            _request: scriberig_project::CadBoundsQueryRequest,
        ) -> Result<scriberig_project::CadBoundsEntityPage, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "cad.entities.bounds".into(),
            })
        }
    };
}
