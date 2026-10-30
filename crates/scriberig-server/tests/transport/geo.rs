//! Verifies geospatial feature pages across the in-process protocol boundary.

use std::collections::BTreeMap;

use scriberig_app::{
    ApplicationContext, ApplicationError, CapabilityAction, CapabilityScope, InspectRequest,
    InspectResult, RuntimeCapability,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_geo::{
    Coordinate, CoordinateReferenceSystem, Feature, FeaturePage, Field, FieldType, GeoDataset,
    GeoLayer, GeoValue, Geometry, GeometryType,
};
use scriberig_project::CandidateValidation;
use scriberig_server::ApplicationService;

/// Serves one authorized geospatial page through the shared protocol contract.
struct GeoFixture {
    /// Stable owning project identity.
    project: ProjectId,
    /// Exact snapshot authorized for this page.
    snapshot: SnapshotId,
    /// Stable semantic geospatial layer identity.
    layer: EntityId,
    /// Stable semantic feature identity used by candidate fixtures.
    feature: EntityId,
}

impl ApplicationService for GeoFixture {
    /// Returns a candidate after checking the fixture's exact target identities.
    ///
    /// # Parameters
    /// - `context`: Trusted exact-snapshot request context.
    /// - `project`: Serialized project identity.
    /// - `path`: Requested project-relative source path.
    /// - `layer`: Serialized semantic layer identity.
    /// - `feature`: Serialized semantic feature identity.
    /// - `_geometry`: Typed replacement geometry.
    ///
    /// # Returns
    /// Durable fixture candidate metadata.
    ///
    /// # Errors
    /// Returns an invalid-request error when the fixture target does not match.
    fn propose_geo_feature_geometry(
        &self,
        context: &ApplicationContext,
        project: &str,
        path: &str,
        layer: &str,
        feature: &str,
        _geometry: &Geometry,
    ) -> Result<scriberig_app::GeoGeometryCandidate, ApplicationError> {
        if context.snapshot != Some(self.snapshot)
            || project != self.project.to_string()
            || path != "parcels.gpkg"
            || layer != self.layer.to_string()
            || feature != self.feature.to_string()
        {
            return Err(ApplicationError::InvalidRequest {
                operation: "geo.geometry.propose".into(),
                message: "fixture target identity mismatch".into(),
            });
        }
        Ok(scriberig_app::GeoGeometryCandidate {
            id: "candidate-1".into(),
            project: self.project,
            base_snapshot: self.snapshot,
            feature: self.feature,
            layer: self.layer,
            path: path.into(),
            validation: CandidateValidation::Pending,
        })
    }

    /// Returns an unsupported result for the unused generic inspection operation.
    ///
    /// # Parameters
    /// - `_context`: Trusted request context.
    /// - `_request`: Generic inspection request.
    ///
    /// # Returns
    /// No inspection result because this operation is unused by the fixture.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        _request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "inspect fixture".into(),
        })
    }

    /// Provides no optional fixture capabilities.
    ///
    /// # Returns
    /// An empty capability list.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        Vec::new()
    }

    /// Returns an unsupported result for the unused operation lifecycle.
    ///
    /// # Parameters
    /// - `_operation_id`: Operation identity requested by the caller.
    ///
    /// # Returns
    /// No operation status because this operation is unused by the fixture.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn operation_status(
        &self,
        _operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation status fixture".into(),
        })
    }

    /// Verifies trusted request context and returns one exact feature page.
    ///
    /// # Parameters
    /// - `context`: Host-injected snapshot and Inspect authority.
    /// - `project`: Serialized project identity.
    /// - `path`: Project-relative `GeoPackage` source path.
    /// - `layer`: Serialized semantic layer identity.
    /// - `offset`: Zero-based page offset.
    /// - `limit`: Requested maximum feature count.
    ///
    /// # Returns
    /// One exact snapshot feature page.
    ///
    /// # Errors
    /// Returns an error if the request does not carry expected host context.
    fn read_geopackage_features(
        &self,
        context: &ApplicationContext,
        project: &str,
        path: &str,
        layer: &str,
        offset: usize,
        limit: usize,
    ) -> Result<scriberig_app::GeoFeaturePage, ApplicationError> {
        assert_eq!(project, self.project.to_string());
        assert_eq!(path, "parcels.gpkg");
        assert_eq!(layer, self.layer.to_string());
        assert_eq!(context.snapshot, Some(self.snapshot));
        assert_eq!(offset, 1);
        assert_eq!(limit, 25);
        assert!(context.capabilities.permits(
            &CapabilityAction::Inspect,
            &CapabilityScope::Custom(format!("project:{}", self.project)),
            Some(self.snapshot),
            Some(scriberig_app::OperationKind::Inspect),
        ));
        let mut attributes = BTreeMap::new();
        attributes.insert("parcel_id".into(), GeoValue::Number("124".into()));
        Ok(scriberig_app::GeoFeaturePage {
            project: self.project,
            snapshot: self.snapshot,
            layer: self.layer,
            page: FeaturePage {
                features: vec![Feature {
                    id: EntityId::from_bytes([9; 16]),
                    layer: self.layer,
                    native_id: Some("124".into()),
                    geometry: Some(Geometry::Point(Coordinate {
                        x: "2.000".into(),
                        y: "3.000".into(),
                        z: None,
                        measure: None,
                    })),
                    attributes,
                }],
                has_more: true,
            },
        })
    }

    /// Serves one typed query result with exact snapshot and source provenance.
    ///
    /// # Parameters
    /// - `context`: Host-injected exact snapshot and Inspect grants.
    /// - `request`: Typed exact-snapshot query.
    ///
    /// # Returns
    /// One bounded feature match and provenance.
    ///
    /// # Errors
    /// Returns an error when project, snapshot, or predicate scope differs.
    fn query_geospatial_features(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::GeoFeatureQuery,
    ) -> Result<scriberig_app::GeoFeatureQueryResult, ApplicationError> {
        assert_eq!(request.project, self.project);
        assert_eq!(request.snapshot, self.snapshot);
        assert_eq!(request.layer, self.layer);
        assert_eq!(request.path, "parcels.gpkg");
        assert_eq!(context.snapshot, Some(self.snapshot));
        assert!(matches!(
            request.predicate,
            scriberig_domain_geo::GeoPredicate::AttributeEquals { .. }
        ));
        let feature = Feature {
            id: self.feature,
            layer: self.layer,
            native_id: Some("124".into()),
            geometry: Some(Geometry::Point(Coordinate {
                x: "2.000".into(),
                y: "3.000".into(),
                z: None,
                measure: None,
            })),
            attributes: BTreeMap::from([("parcel_id".into(), GeoValue::Number("124".into()))]),
        };
        Ok(scriberig_app::GeoFeatureQueryResult {
            project: self.project,
            page: scriberig_project::GeoPackageFeatureQueryPage {
                snapshot: self.snapshot,
                layer: self.layer,
                features: vec![feature],
                has_more: false,
                provenance: scriberig_project::GeoQueryProvenance {
                    source_revision: scriberig_core::RevisionId::new([8; 32]),
                    source_digest: [0xab; 32],
                    provider: "geopackage".into(),
                    layer_crs: Some(CoordinateReferenceSystem {
                        authority: Some("EPSG".into()),
                        code: Some("4326".into()),
                        definition: Some("EPSG:4326".into()),
                    }),
                    strategy: scriberig_project::GeoQueryStrategy::StreamedSourcePages,
                    coordinate_transform: None,
                    diagnostics: vec!["fixture provenance".into()],
                },
            },
        })
    }

    /// Verifies trusted catalog context and returns one bounded semantic layer summary.
    ///
    /// # Parameters
    /// - `context`: Host-injected snapshot and Inspect authority.
    /// - `project`: Serialized project identity.
    /// - `path`: Project-relative `GeoPackage` source path.
    /// - `offset`: Zero-based layer offset.
    /// - `limit`: Requested maximum layer count.
    ///
    /// # Returns
    /// Exact dataset and layer catalog page.
    ///
    /// # Errors
    /// Returns an error if the request does not carry expected host context.
    fn inspect_geopackage(
        &self,
        context: &ApplicationContext,
        project: &str,
        path: &str,
        offset: usize,
        limit: usize,
    ) -> Result<scriberig_app::GeoPackageCatalogPage, ApplicationError> {
        assert_eq!(project, self.project.to_string());
        assert_eq!(path, "parcels.gpkg");
        assert_eq!(context.snapshot, Some(self.snapshot));
        assert_eq!(offset, 0);
        assert_eq!(limit, 10);
        assert!(context.capabilities.permits(
            &CapabilityAction::Inspect,
            &CapabilityScope::Custom(format!("project:{}", self.project)),
            Some(self.snapshot),
            Some(scriberig_app::OperationKind::Inspect),
        ));
        let dataset = EntityId::from_bytes([8; 16]);
        Ok(scriberig_app::GeoPackageCatalogPage {
            project: self.project,
            snapshot: self.snapshot,
            dataset: GeoDataset {
                id: dataset,
                layers: vec![self.layer],
            },
            source_digest: [0xab; 32],
            layers: vec![scriberig_project::GeoPackageLayerSummary {
                layer: GeoLayer {
                    id: self.layer,
                    dataset,
                    name: "parcels".into(),
                    fields: vec![Field {
                        name: "parcel_id".into(),
                        field_type: FieldType::Number,
                        nullable: false,
                    }],
                    crs: Some(CoordinateReferenceSystem {
                        authority: Some("EPSG".into()),
                        code: Some("4326".into()),
                        definition: Some("EPSG:4326".into()),
                    }),
                    geometry_type: Some(GeometryType::Point),
                },
                feature_count: 2,
                extent: Some(["1".into(), "2".into(), "3".into(), "4".into()]),
                editable: false,
                diagnostics: vec!["package writer unavailable".into()],
            }],
            total: 1,
            has_more: false,
        })
    }
}

#[path = "geo/catalog.rs"]
mod catalog;
#[path = "geo/feature_page.rs"]
mod feature_page;
#[path = "geo/geometry_candidate.rs"]
mod geometry_candidate;
#[path = "geo/query.rs"]
mod query;
