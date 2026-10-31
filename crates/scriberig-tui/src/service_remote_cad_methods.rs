//! Defines cad and bounded geospatial methods for the remote TUI service.

macro_rules! remote_cad_methods {
    () => {
        /// Reads an exact-snapshot CAD drawing through the typed remote client.
        ///
        /// # Parameters
        /// - `_context`: Authorization is applied by the remote host.
        /// - `request`: Exact project, snapshot, and drawing identity.
        ///
        /// # Returns
        /// Typed drawing metadata and semantic layers.
        ///
        /// # Errors
        /// Returns transport errors or a response outside the requested scope.
        fn inspect_cad_drawing(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_protocol::CadDrawingInspectRequest,
        ) -> Result<scriberig_protocol::CadDrawingInspectResponse, ApplicationError> {
            let response = self
                .client
                .inspect_cad_drawing(&request.project, &request.snapshot, &request.drawing)
                .map_err(remote_error)?;
            if response.project != request.project
                || response.snapshot != request.snapshot
                || response.drawing != request.drawing
            {
                return Err(ApplicationError::Internal {
                    message: "remote CAD drawing response does not match the exact request"
                        .to_owned(),
                });
            }
            Ok(response)
        }

        /// Reads a bounded CAD entity page through the typed remote client.
        ///
        /// # Parameters
        /// - `_context`: Authorization is applied by the remote host.
        /// - `request`: Exact drawing identity and entity page bounds.
        ///
        /// # Returns
        /// Typed semantic entities and continuation metadata.
        ///
        /// # Errors
        /// Returns transport errors or a response outside the requested scope.
        fn inspect_cad_entities(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_protocol::CadEntitiesRequest,
        ) -> Result<scriberig_protocol::CadEntitiesResponse, ApplicationError> {
            let response = self
                .client
                .cad_entities(
                    &request.project,
                    &request.snapshot,
                    &request.drawing,
                    request.offset,
                    request.limit,
                )
                .map_err(remote_error)?;
            if response.project != request.project
                || response.snapshot != request.snapshot
                || response.drawing != request.drawing
            {
                return Err(ApplicationError::Internal {
                    message: "remote CAD entity page does not match the exact request".to_owned(),
                });
            }
            Ok(response)
        }

        /// Proposes a layer change through the typed remote candidate operation.
        ///
        /// # Parameters
        /// - `_context`: Authorization is applied by the remote host.
        /// - `request`: Exact project, snapshot, entity, and destination layer.
        ///
        /// # Returns
        /// Durable candidate metadata for the common Changes review.
        ///
        /// # Errors
        /// Returns transport errors or a candidate outside the requested base scope.
        fn propose_cad_layer_change(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_protocol::CadLayerChangeRequest,
        ) -> Result<scriberig_protocol::CadLayerChangeResponse, ApplicationError> {
            let response = self
                .client
                .propose_cad_layer_change(
                    &request.project,
                    &request.snapshot,
                    &request.entity,
                    &request.destination_layer,
                )
                .map_err(remote_error)?;
            if response.project != request.project || response.base_snapshot != request.snapshot {
                return Err(ApplicationError::Internal {
                    message: "remote CAD candidate does not match the exact request base"
                        .to_owned(),
                });
            }
            Ok(response)
        }

        /// Runs a bounded typed geospatial query through the remote client.
        ///
        /// # Parameters
        /// - `_context`: Authorization is applied by the remote host.
        /// - `request`: Exact query scope, predicate, and result bounds.
        ///
        /// # Returns
        /// Typed query matches with exact source provenance.
        ///
        /// # Errors
        /// Returns transport errors or a response outside the requested scope.
        fn query_geospatial_features(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::GeoFeatureQuery,
        ) -> Result<scriberig_protocol::GeoFeatureQueryResponse, ApplicationError> {
            let response = self
                .client
                .query_geospatial_features(scriberig_protocol::GeoFeatureQueryRequest {
                    project: request.project.to_string(),
                    path: request.path.clone(),
                    snapshot: request.snapshot.to_string(),
                    layer: request.layer.to_string(),
                    predicate: crate::view::geo::predicate_to_protocol(&request.predicate),
                    offset: request.offset,
                    limit: request.limit,
                })
                .map_err(remote_error)?;
            if response.project != request.project.to_string()
                || response.snapshot != request.snapshot.to_string()
                || response.layer != request.layer.to_string()
            {
                return Err(ApplicationError::Internal {
                    message: "remote geospatial query result does not match the exact request"
                        .to_owned(),
                });
            }
            Ok(response)
        }
    };
}
