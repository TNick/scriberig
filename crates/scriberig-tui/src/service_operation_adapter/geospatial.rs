//! Provides geospatial methods for the application operation adapter.

macro_rules! operation_geospatial_methods {
    () => {
        /// Reads a bounded `GeoPackage` feature page through the authorized application API.
        ///
        /// # Parameters
        /// - `context`: Caller grants and application context.
        /// - `project`: Stable owning project identity.
        /// - `path`: Exact project-relative artifact path.
        /// - `snapshot`: Exact immutable source snapshot.
        /// - `layer`: Stable semantic layer identity.
        /// - `offset`: Zero-based feature offset.
        ///
        /// # Returns
        /// Protocol-shaped feature page with exact identities retained.
        ///
        /// # Errors
        /// Returns authorization, source, snapshot, or normalization errors.
        fn read_geopackage_features(
            &self,
            context: &ApplicationContext,
            project: scriberig_core::ProjectId,
            path: &str,
            snapshot: SnapshotId,
            layer: EntityId,
            offset: usize,
        ) -> Result<scriberig_protocol::GeoFeaturesResponse, ApplicationError> {
            self.service
                .read_geopackage_features(
                    &context.clone().with_snapshot(snapshot),
                    project,
                    path,
                    layer,
                    offset,
                    40,
                )
                .map(crate::view::geo::features_from_application)
        }

        /// Proposes a typed `GeoPackage` attribute edit through the application layer.
        ///
        /// # Parameters
        /// - `context`: Caller grants and application context.
        /// - `edit`: Exact source, feature, field, and typed replacement.
        ///
        /// # Returns
        /// Durable candidate metadata for the common Changes view.
        ///
        /// # Errors
        /// Returns authorization, stale-base, schema, or candidate errors.
        fn propose_geo_attribute_edit(
            &self,
            context: &ApplicationContext,
            edit: &scriberig_project::GeoPackageFeatureAttributeEdit,
        ) -> Result<scriberig_app::GeoAttributeCandidate, ApplicationError> {
            self.service.propose_geo_feature_attribute(
                &context.clone().with_snapshot(edit.base_snapshot),
                edit.clone(),
            )
        }

        /// Proposes an exact-base `GeoPackage` geometry edit through the application service.
        ///
        /// # Parameters
        /// - `context`: Caller grants and application context.
        /// - `request`: Exact project, source, snapshot, feature, and typed geometry.
        ///
        /// # Returns
        /// Durable candidate metadata for common review and apply.
        ///
        /// # Errors
        /// Returns authorization, stale-base, unsupported-geometry, or candidate errors.
        fn propose_geo_geometry_edit(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::GeoGeometryEditRequest,
        ) -> Result<scriberig_app::GeoGeometryCandidate, ApplicationError> {
            let project =
                request
                    .project
                    .parse()
                    .map_err(|_| ApplicationError::InvalidRequest {
                        operation: "geo.geometry.propose".to_owned(),
                        message: "project identity is invalid".to_owned(),
                    })?;
            let snapshot =
                request
                    .snapshot
                    .parse()
                    .map_err(|_| ApplicationError::InvalidRequest {
                        operation: "geo.geometry.propose".to_owned(),
                        message: "snapshot identity is invalid".to_owned(),
                    })?;
            let layer = request
                .layer
                .parse()
                .map_err(|_| ApplicationError::InvalidRequest {
                    operation: "geo.geometry.propose".to_owned(),
                    message: "layer identity is invalid".to_owned(),
                })?;
            let feature =
                request
                    .feature
                    .parse()
                    .map_err(|_| ApplicationError::InvalidRequest {
                        operation: "geo.geometry.propose".to_owned(),
                        message: "feature identity is invalid".to_owned(),
                    })?;
            self.service.propose_geo_feature_geometry(
                &context.clone().with_snapshot(snapshot),
                project,
                &request.path,
                layer,
                feature,
                &crate::view::geo::geometry_to_application(request.geometry.clone()),
            )
        }
    };
}
