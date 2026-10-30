//! Adapts typed geospatial reads to the shared application service.

macro_rules! geo_methods {
    () => {
        /// Reads a bounded `GeoPackage` catalog through the shared application boundary.
        ///
        /// # Parameters
        /// - `context`: Trusted request context containing the exact snapshot and grants.
        /// - `project`: Serialized stable project identity.
        /// - `path`: Project-relative `GeoPackage` path from persisted scan evidence.
        /// - `offset`: Zero-based layer offset.
        /// - `limit`: Maximum layer page size.
        ///
        /// # Returns
        /// Provider-neutral dataset and layer catalog page.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project read errors.
        fn inspect_geopackage(
            &self,
            context: &ApplicationContext,
            project: &str,
            path: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::GeoPackageCatalogPage, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "geo.catalog.inspect".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            scriberig_app::ApplicationService::inspect_geopackage(
                self, context, project, path, offset, limit,
            )
        }

        /// Reads bounded `GeoPackage` features through the shared application boundary.
        ///
        /// # Parameters
        /// - `context`: Trusted request context containing the exact snapshot and grants.
        /// - `project`: Serialized stable project identity.
        /// - `path`: Project-relative `GeoPackage` path from persisted scan evidence.
        /// - `layer`: Serialized stable semantic layer identity.
        /// - `offset`: Zero-based feature offset.
        /// - `limit`: Maximum feature page size.
        ///
        /// Provider-neutral feature page tied to the exact snapshot.
        ///
        /// # Returns
        /// The bounded feature page.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project read errors.
        fn read_geopackage_features(
            &self,
            context: &ApplicationContext,
            project: &str,
            path: &str,
            layer: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::GeoFeaturePage, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "geo.features.read".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let layer = layer
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "geo.features.read".into(),
                    message: format!("invalid layer identity: {error}"),
                })?;
            scriberig_app::ApplicationService::read_geopackage_features(
                self, context, project, path, layer, offset, limit,
            )
        }

        /// Reads exact-snapshot QGIS layer references through shared Inspect scope.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with caller grants.
        /// - `project`: Serialized owning project identity.
        /// - `path`: Exact project-relative QGS/QGZ path.
        ///
        /// # Returns
        /// QGIS project semantics with resolved shared `GeoLayer` identities.
        ///
        /// # Errors
        /// Returns invalid project identity, authorization, or semantic read errors.
        fn inspect_qgis_project(
            &self,
            context: &ApplicationContext,
            project: &str,
            path: &str,
        ) -> Result<scriberig_app::QgisProjectResult, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "qgis.project.inspect".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let snapshot = context
                .snapshot
                .ok_or_else(|| ApplicationError::InvalidRequest {
                    operation: "qgis.project.inspect".into(),
                    message: "QGIS project inspection requires an exact snapshot".into(),
                })?;
            scriberig_app::QueryService::inspect_qgis_project(
                self,
                context,
                &scriberig_app::QgisProjectQuery {
                    project,
                    snapshot,
                    path: path.to_owned(),
                },
            )
        }

        /// Proposes a QGIS layer-reference rename through exact-snapshot application authority.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with exact snapshot and Propose grant.
        /// - `project`: Serialized owning project identity.
        /// - `reference`: Serialized semantic QGIS layer-reference identity.
        /// - `new_name`: New authored display name.
        ///
        /// # Returns
        /// Candidate metadata for shared validation, review, and apply.
        ///
        /// # Errors
        /// Returns invalid identities, authorization, or project candidate errors.
        fn propose_qgis_layer_rename(
            &self,
            context: &ApplicationContext,
            project: &str,
            reference: &str,
            new_name: &str,
        ) -> Result<scriberig_app::QgisLayerRenameCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "qgis.layer.rename".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let reference =
                reference
                    .parse()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "qgis.layer.rename".into(),
                        message: format!("invalid layer-reference identity: {error}"),
                    })?;
            scriberig_app::ApplicationService::propose_qgis_layer_rename(
                self, context, project, reference, new_name,
            )
        }

        /// Proposes a complete QGIS layer order through exact-snapshot application authority.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with project-scoped Propose grant.
        /// - `project`: Serialized owning project identity.
        /// - `path`: Exact project-relative QGS/QGZ path.
        /// - `ordered_references`: Complete ordered semantic layer identities.
        ///
        /// # Returns
        /// Candidate metadata for shared validation, review, and apply.
        ///
        /// # Errors
        /// Returns invalid identities, authorization, or project candidate errors.
        fn propose_qgis_layer_order(
            &self,
            context: &ApplicationContext,
            project: &str,
            path: &str,
            ordered_references: &[String],
        ) -> Result<scriberig_app::QgisLayerOrderCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "qgis.layer_order.reorder".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let ordered_references = ordered_references
                .iter()
                .map(|reference| {
                    reference
                        .parse()
                        .map_err(|error| ApplicationError::InvalidRequest {
                            operation: "qgis.layer_order.reorder".into(),
                            message: format!("invalid layer-reference identity: {error}"),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            scriberig_app::ApplicationService::propose_qgis_layer_order(
                self,
                context,
                project,
                path,
                &ordered_references,
            )
        }

        /// Executes an authorized exact-snapshot bounded geospatial query.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with caller grants.
        /// - `request`: Typed query criteria and exact source identities.
        ///
        /// # Returns
        /// Matching feature summaries with exact source provenance.
        ///
        /// # Errors
        /// Returns authorization, validation, or project query errors.
        fn query_geospatial_features(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::GeoFeatureQuery,
        ) -> Result<scriberig_app::GeoFeatureQueryResult, ApplicationError> {
            scriberig_app::QueryService::query_geospatial_features(self, context, request)
        }

        /// Proposes one typed feature geometry through shared application authorization.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot request context and grants.
        /// - `project`: Serialized stable project identity.
        /// - `path`: Project-relative `GeoPackage` path.
        /// - `layer`: Serialized semantic layer identity.
        /// - `feature`: Serialized semantic feature identity.
        /// - `geometry`: Typed replacement geometry in layer CRS.
        ///
        /// # Returns
        /// Durable candidate metadata without package bytes.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or candidate errors.
        fn propose_geo_feature_geometry(
            &self,
            context: &ApplicationContext,
            project: &str,
            path: &str,
            layer: &str,
            feature: &str,
            geometry: &scriberig_domain_geo::Geometry,
        ) -> Result<scriberig_app::GeoGeometryCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "geo.geometry.propose".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let layer = layer
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "geo.geometry.propose".into(),
                    message: format!("invalid layer identity: {error}"),
                })?;
            let feature = feature
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "geo.geometry.propose".into(),
                    message: format!("invalid feature identity: {error}"),
                })?;
            scriberig_app::ApplicationService::propose_geo_feature_geometry(
                self, context, project, path, layer, feature, geometry,
            )
        }

        /// Proposes one typed feature attribute through shared application authorization.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot request context and grants.
        /// - `edit`: Exact project, source, feature, field, and typed value.
        ///
        /// # Returns
        /// Durable candidate metadata without package bytes.
        ///
        /// # Errors
        /// Returns authorization, malformed identity, or candidate errors.
        fn propose_geo_feature_attribute(
            &self,
            context: &ApplicationContext,
            edit: scriberig_project::GeoPackageFeatureAttributeEdit,
        ) -> Result<scriberig_app::GeoAttributeCandidate, ApplicationError> {
            scriberig_app::ApplicationService::propose_geo_feature_attribute(self, context, edit)
        }
    };
}
