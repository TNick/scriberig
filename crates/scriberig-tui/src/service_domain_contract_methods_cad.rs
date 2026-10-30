//! Declares default CAD and geospatial methods for the shared TUI service contract.

macro_rules! tui_cad_domain_methods {
    () => {
        /// Reads one exact-snapshot CAD drawing summary.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, and drawing identity.
        ///
        /// # Returns
        /// Typed drawing metadata and semantic layers.
        ///
        /// # Errors
        /// Returns authorization, unsupported-operation, snapshot, or transport errors.
        fn inspect_cad_drawing(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::CadDrawingInspectRequest,
        ) -> Result<scriberig_protocol::CadDrawingInspectResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "CAD drawing inspection".to_owned(),
            })
        }

        /// Reads one bounded exact-snapshot CAD entity page.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact drawing identity and entity page bounds.
        ///
        /// # Returns
        /// Typed semantic entities with continuation metadata.
        ///
        /// # Errors
        /// Returns authorization, unsupported-operation, snapshot, or transport errors.
        fn inspect_cad_entities(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::CadEntitiesRequest,
        ) -> Result<scriberig_protocol::CadEntitiesResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "CAD entity inspection".to_owned(),
            })
        }

        /// Proposes a layer change through the shared CAD candidate operation.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, entity, and destination layer.
        ///
        /// # Returns
        /// Durable candidate metadata for shared review and apply.
        ///
        /// # Errors
        /// Returns authorization, unsupported-operation, stale-base, or transport errors.
        fn propose_cad_layer_change(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::CadLayerChangeRequest,
        ) -> Result<scriberig_protocol::CadLayerChangeResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "CAD layer change candidate".to_owned(),
            })
        }

        /// Reads one bounded `GeoPackage` layer catalog at an exact snapshot.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_project`: Owning project.
        /// - `_path`: Exact project-relative artifact path.
        /// - `_snapshot`: Exact immutable semantic snapshot.
        /// - `_offset`: Zero-based catalog offset.
        ///
        /// # Returns
        /// Typed dataset and layer metadata with continuation state.
        ///
        /// # Errors
        /// Returns authorization, source lookup, snapshot, or transport errors.
        fn inspect_geopackage(
            &self,
            _context: &ApplicationContext,
            _project: scriberig_core::ProjectId,
            _path: &str,
            _snapshot: SnapshotId,
            _offset: usize,
        ) -> Result<scriberig_protocol::GeoCatalogResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "GeoPackage catalog inspection".to_owned(),
            })
        }

        /// Reads one bounded `GeoPackage` feature page at an exact snapshot.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_project`: Owning project.
        /// - `_path`: Exact project-relative artifact path.
        /// - `_snapshot`: Exact immutable semantic snapshot.
        /// - `_layer`: Stable semantic layer identity.
        /// - `_offset`: Zero-based feature offset.
        ///
        /// # Returns
        /// Provider-neutral typed features and continuation state.
        ///
        /// # Errors
        /// Returns authorization, source lookup, snapshot, or transport errors.
        fn read_geopackage_features(
            &self,
            _context: &ApplicationContext,
            _project: scriberig_core::ProjectId,
            _path: &str,
            _snapshot: SnapshotId,
            _layer: EntityId,
            _offset: usize,
        ) -> Result<scriberig_protocol::GeoFeaturesResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "GeoPackage feature inspection".to_owned(),
            })
        }

        /// Runs one bounded typed geospatial query under the exact snapshot scope.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, source, predicate, and page bounds.
        ///
        /// # Returns
        /// Matching typed features with source and CRS provenance.
        ///
        /// # Errors
        /// Returns authorization, source, predicate, or transport errors.
        fn query_geospatial_features(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::GeoFeatureQuery,
        ) -> Result<scriberig_protocol::GeoFeatureQueryResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "GeoPackage feature query".to_owned(),
            })
        }

        /// Reads an exact-snapshot QGIS project and its resolved `GeoLayer` links.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, project path, and snapshot.
        ///
        /// # Returns
        /// Provider-neutral QGIS references and normalized source paths.
        ///
        /// # Errors
        /// Returns authorization, source lookup, snapshot, or transport errors.
        fn inspect_qgis_project(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::QgisProjectQuery,
        ) -> Result<scriberig_protocol::QgisProjectResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "QGIS project inspection".to_owned(),
            })
        }

        /// Sets or clears the caller's personal source projection style.
        ///
        /// # Parameters
        /// - `_context`: Trusted caller and authorization context.
        /// - `_request`: User-scoped style value or clear operation.
        ///
        /// # Returns
        /// The authoritative stored style and revision.
        ///
        /// # Errors
        /// Returns permission, style validation, revision conflict, or transport errors.
        fn set_projection_style_preference(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::ProjectionStyleSettingRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleSettingResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "personal projection style preference".to_owned(),
            })
        }

        /// Resolves the effective style, provenance, and fidelity for a view.
        ///
        /// # Parameters
        /// - `_context`: Trusted caller and authorization context.
        /// - `_request`: Exact projection kind, scope, view, and purpose.
        ///
        /// # Returns
        /// The resolved canonical style and its evidence.
        ///
        /// # Errors
        /// Returns style validation, configuration, or transport errors.
        fn resolve_projection_style(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::ProjectionStyleResolveRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleResolveResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "effective projection style resolution".to_owned(),
            })
        }

        /// Proposes an exact-snapshot QGIS layer-reference rename.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, reference, and new display name.
        ///
        /// # Returns
        /// Durable candidate metadata for common review and apply.
        ///
        /// # Errors
        /// Returns authorization, stale-base, candidate, or transport errors.
        fn propose_qgis_layer_rename(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::QgisLayerRenameRequest,
        ) -> Result<scriberig_protocol::QgisLayerRenameResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "QGIS layer rename candidate".to_owned(),
            })
        }

        /// Proposes a complete exact-snapshot QGIS project layer order.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, source path, and full permutation.
        ///
        /// # Returns
        /// Durable candidate metadata for common review and apply.
        ///
        /// # Errors
        /// Returns authorization, stale-base, candidate, or transport errors.
        fn propose_qgis_layer_order(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::QgisLayerOrderRequest,
        ) -> Result<scriberig_protocol::QgisLayerOrderResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "QGIS layer-order candidate".to_owned(),
            })
        }

        /// Proposes one typed `GeoPackage` attribute edit through shared candidate operations.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_edit`: Exact project, snapshot, layer, feature, field, and value.
        ///
        /// # Returns
        /// Durable candidate metadata for the common review and apply flow.
        ///
        /// # Errors
        /// Returns authorization, stale-base, schema, or transport errors.
        fn propose_geo_attribute_edit(
            &self,
            _context: &ApplicationContext,
            _edit: &scriberig_project::GeoPackageFeatureAttributeEdit,
        ) -> Result<scriberig_app::GeoAttributeCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "GeoPackage attribute candidate".to_owned(),
            })
        }

        /// Proposes a typed exact-base `GeoPackage` geometry edit.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, source, snapshot, layer, feature, and geometry.
        ///
        /// # Returns
        /// Durable candidate metadata for shared validation and review.
        ///
        /// # Errors
        /// Returns authorization, stale-base, unsupported-geometry, or transport errors.
        fn propose_geo_geometry_edit(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::GeoGeometryEditRequest,
        ) -> Result<scriberig_app::GeoGeometryCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "GeoPackage geometry candidate".to_owned(),
            })
        }
    };
}
