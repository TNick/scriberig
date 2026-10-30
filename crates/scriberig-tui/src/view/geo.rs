//! Normalizes exact-snapshot geospatial reads into terminal-safe protocol views.

use scriberig_protocol::{
    GeoCoordinate, GeoCrsView, GeoFeatureView, GeoFeaturesResponse, GeoFieldType,
    GeoFieldView as FieldView, GeoGeometry, GeoGeometryType, GeoLayerView, GeoValue,
};
use std::collections::BTreeMap;

#[path = "geo/query.rs"]
mod query;
pub use query::{predicate_to_protocol, query_from_application};

/// Converts an authorized application catalog page to its wire-shaped view.
///
/// # Parameters
/// - `page`: Exact-snapshot catalog page returned by the application service.
///
/// # Returns
/// Stable dataset and layer IDs, source digest evidence, and bounded metadata.
pub fn catalog_from_application(
    page: scriberig_app::GeoPackageCatalogPage,
) -> scriberig_protocol::GeoCatalogResponse {
    scriberig_protocol::GeoCatalogResponse {
        project: page.project.to_string(),
        snapshot: page.snapshot.to_string(),
        dataset: page.dataset.id.to_string(),
        source_digest: hex_digest(&page.source_digest),
        layers: page
            .layers
            .into_iter()
            .map(layer_from_application)
            .collect(),
        total: page.total,
        has_more: page.has_more,
    }
}

/// Converts an authorized application feature page to its wire-shaped view.
///
/// # Parameters
/// - `page`: Exact-snapshot provider-neutral feature page.
///
/// # Returns
/// Feature identities, source IDs, geometry, and typed attributes.
#[must_use]
pub fn features_from_application(page: scriberig_app::GeoFeaturePage) -> GeoFeaturesResponse {
    GeoFeaturesResponse {
        project: page.project.to_string(),
        snapshot: page.snapshot.to_string(),
        layer: page.layer.to_string(),
        features: page
            .page
            .features
            .into_iter()
            .map(|feature| GeoFeatureView {
                id: feature.id.to_string(),
                native_id: feature.native_id,
                geometry: feature.geometry.map(geometry_from_application),
                attributes: feature
                    .attributes
                    .into_iter()
                    .map(|(name, value)| (name, value_from_application(value)))
                    .collect::<BTreeMap<_, _>>(),
            })
            .collect(),
        has_more: page.page.has_more,
    }
}

/// Converts one application layer summary while retaining schema and CRS.
///
/// # Parameters
/// - `summary`: Bounded project layer metadata.
///
/// # Returns
/// Protocol layer view with exact IDs and source diagnostics.
fn layer_from_application(summary: scriberig_project::GeoPackageLayerSummary) -> GeoLayerView {
    let layer = summary.layer;
    GeoLayerView {
        id: layer.id.to_string(),
        dataset: layer.dataset.to_string(),
        name: layer.name,
        fields: layer
            .fields
            .into_iter()
            .map(field_from_application)
            .collect(),
        crs: layer.crs.map(|crs| GeoCrsView {
            authority: crs.authority,
            code: crs.code,
            definition: crs.definition,
        }),
        geometry_type: layer.geometry_type.map(|kind| match kind {
            scriberig_domain_geo::GeometryType::Point => GeoGeometryType::Point,
            scriberig_domain_geo::GeometryType::LineString => GeoGeometryType::LineString,
            scriberig_domain_geo::GeometryType::Polygon => GeoGeometryType::Polygon,
            scriberig_domain_geo::GeometryType::GeometryCollection => {
                GeoGeometryType::GeometryCollection
            }
        }),
        feature_count: summary.feature_count,
        extent: summary.extent,
        editable: summary.editable,
        diagnostics: summary.diagnostics,
    }
}

/// Converts one application schema field.
///
/// # Parameters
/// - `field`: Provider-neutral semantic field definition.
///
/// # Returns
/// Wire field preserving authored name, scalar type, and nullability.
fn field_from_application(field: scriberig_domain_geo::Field) -> FieldView {
    FieldView {
        name: field.name,
        field_type: match field.field_type {
            scriberig_domain_geo::FieldType::Text => GeoFieldType::Text,
            scriberig_domain_geo::FieldType::Number => GeoFieldType::Number,
            scriberig_domain_geo::FieldType::Boolean => GeoFieldType::Boolean,
            scriberig_domain_geo::FieldType::DateTime => GeoFieldType::DateTime,
            scriberig_domain_geo::FieldType::Binary => GeoFieldType::Binary,
        },
        nullable: field.nullable,
    }
}

/// Converts one domain geometry recursively without changing coordinate text.
///
/// # Parameters
/// - `geometry`: Canonical provider-neutral domain geometry.
///
/// # Returns
/// Equivalent protocol geometry in exact coordinate text.
fn geometry_from_application(geometry: scriberig_domain_geo::Geometry) -> GeoGeometry {
    match geometry {
        scriberig_domain_geo::Geometry::Point(point) => {
            GeoGeometry::Point(coordinate_from_application(point))
        }
        scriberig_domain_geo::Geometry::LineString(points) => GeoGeometry::LineString(
            points
                .into_iter()
                .map(coordinate_from_application)
                .collect(),
        ),
        scriberig_domain_geo::Geometry::Polygon(rings) => GeoGeometry::Polygon(
            rings
                .into_iter()
                .map(|ring| ring.into_iter().map(coordinate_from_application).collect())
                .collect(),
        ),
        scriberig_domain_geo::Geometry::Collection(parts) => {
            GeoGeometry::Collection(parts.into_iter().map(geometry_from_application).collect())
        }
    }
}

/// Converts protocol geometry into canonical provider-neutral application geometry.
///
/// # Parameters
/// - `geometry`: Typed exact coordinate geometry from a candidate request.
///
/// # Returns
/// Domain geometry with every coordinate string preserved.
#[must_use]
pub fn geometry_to_application(geometry: GeoGeometry) -> scriberig_domain_geo::Geometry {
    match geometry {
        GeoGeometry::Point(point) => {
            scriberig_domain_geo::Geometry::Point(coordinate_to_application(point))
        }
        GeoGeometry::LineString(points) => scriberig_domain_geo::Geometry::LineString(
            points.into_iter().map(coordinate_to_application).collect(),
        ),
        GeoGeometry::Polygon(rings) => scriberig_domain_geo::Geometry::Polygon(
            rings
                .into_iter()
                .map(|ring| ring.into_iter().map(coordinate_to_application).collect())
                .collect(),
        ),
        GeoGeometry::Collection(parts) => scriberig_domain_geo::Geometry::Collection(
            parts.into_iter().map(geometry_to_application).collect(),
        ),
    }
}

/// Converts one exact domain coordinate.
///
/// # Parameters
/// - `coordinate`: Domain coordinate with optional z and measure ordinates.
///
/// # Returns
/// Protocol coordinate with every authored decimal string preserved.
fn coordinate_from_application(coordinate: scriberig_domain_geo::Coordinate) -> GeoCoordinate {
    GeoCoordinate {
        x: coordinate.x,
        y: coordinate.y,
        z: coordinate.z,
        measure: coordinate.measure,
    }
}

/// Converts one protocol coordinate into its exact domain representation.
///
/// # Parameters
/// - `coordinate`: Typed protocol coordinate.
///
/// # Returns
/// Domain coordinate with optional ordinates preserved.
fn coordinate_to_application(coordinate: GeoCoordinate) -> scriberig_domain_geo::Coordinate {
    scriberig_domain_geo::Coordinate {
        x: coordinate.x,
        y: coordinate.y,
        z: coordinate.z,
        measure: coordinate.measure,
    }
}

/// Converts one typed domain attribute value.
///
/// # Parameters
/// - `value`: Domain attribute value.
///
/// # Returns
/// Wire value with exact numeric and date/time strings preserved.
fn value_from_application(value: scriberig_domain_geo::GeoValue) -> GeoValue {
    match value {
        scriberig_domain_geo::GeoValue::Null => GeoValue::Null,
        scriberig_domain_geo::GeoValue::Text(text) => GeoValue::Text(text),
        scriberig_domain_geo::GeoValue::Number(number) => GeoValue::Number(number),
        scriberig_domain_geo::GeoValue::Boolean(boolean) => GeoValue::Boolean(boolean),
        scriberig_domain_geo::GeoValue::DateTime(datetime) => GeoValue::DateTime(datetime),
    }
}

/// Converts a typed domain value to its protocol representation for a candidate request.
///
/// # Parameters
/// - `value`: Provider-neutral domain value.
///
/// # Returns
/// Equivalent typed protocol value.
#[must_use]
pub fn value_to_protocol(value: scriberig_domain_geo::GeoValue) -> GeoValue {
    value_from_application(value)
}

/// Converts one typed protocol attribute value into the geospatial domain value.
///
/// # Parameters
/// - `value`: Provider-neutral wire scalar.
///
/// # Returns
/// Equivalent semantic attribute value.
#[must_use]
pub fn value_from_protocol(value: GeoValue) -> scriberig_domain_geo::GeoValue {
    match value {
        GeoValue::Null => scriberig_domain_geo::GeoValue::Null,
        GeoValue::Text(value) => scriberig_domain_geo::GeoValue::Text(value),
        GeoValue::Number(value) => scriberig_domain_geo::GeoValue::Number(value),
        GeoValue::Boolean(value) => scriberig_domain_geo::GeoValue::Boolean(value),
        GeoValue::DateTime(value) => scriberig_domain_geo::GeoValue::DateTime(value),
    }
}

/// Encodes source digest bytes as lowercase hexadecimal without another dependency.
///
/// # Parameters
/// - `digest`: Exact BLAKE3 digest bytes.
///
/// # Returns
/// A 64-character lowercase hexadecimal string.
fn hex_digest(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(64);
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::features_from_application;
    use scriberig_core::{EntityId, ProjectId, SnapshotId};
    use scriberig_domain_geo::{Coordinate, Feature, FeaturePage, GeoValue, Geometry};
    use std::collections::BTreeMap;

    /// Preserves exact geometry coordinates and typed attributes in the terminal protocol view.
    #[test]
    fn feature_mapping_preserves_semantic_and_source_identity() {
        let layer = EntityId::from_bytes([1; 16]);
        let feature = EntityId::from_bytes([2; 16]);
        let mut attributes = BTreeMap::new();
        attributes.insert("parcel_id".to_owned(), GeoValue::Number("124".to_owned()));
        let page = scriberig_app::GeoFeaturePage {
            project: ProjectId::from_bytes([3; 16]),
            snapshot: SnapshotId::new([4; 32]),
            layer,
            page: FeaturePage {
                features: vec![Feature {
                    id: feature,
                    layer,
                    native_id: Some("provider-row-8".to_owned()),
                    geometry: Some(Geometry::Point(Coordinate {
                        x: "12.3400".to_owned(),
                        y: "56.7800".to_owned(),
                        z: Some("9.00".to_owned()),
                        measure: None,
                    })),
                    attributes,
                }],
                has_more: true,
            },
        };
        let response = features_from_application(page);
        assert_eq!(response.layer, layer.to_string());
        assert_eq!(response.features[0].id, feature.to_string());
        assert_eq!(
            response.features[0].native_id.as_deref(),
            Some("provider-row-8")
        );
        assert_eq!(
            response.features[0].geometry,
            Some(scriberig_protocol::GeoGeometry::Point(
                scriberig_protocol::GeoCoordinate {
                    x: "12.3400".to_owned(),
                    y: "56.7800".to_owned(),
                    z: Some("9.00".to_owned()),
                    measure: None,
                }
            ))
        );
        assert_eq!(
            response.features[0].attributes.get("parcel_id"),
            Some(&scriberig_protocol::GeoValue::Number("124".to_owned()))
        );
        assert!(response.has_more);
    }

    /// Preserves exact decimal coordinates when converting an edit payload to domain geometry.
    #[test]
    fn geometry_edit_mapping_keeps_coordinate_lexemes() {
        let geometry = scriberig_protocol::GeoGeometry::Point(scriberig_protocol::GeoCoordinate {
            x: "1.2300".to_owned(),
            y: "-0.0050".to_owned(),
            z: Some("10.00".to_owned()),
            measure: Some("7".to_owned()),
        });
        let domain = super::geometry_to_application(geometry);
        assert_eq!(
            domain,
            scriberig_domain_geo::Geometry::Point(scriberig_domain_geo::Coordinate {
                x: "1.2300".to_owned(),
                y: "-0.0050".to_owned(),
                z: Some("10.00".to_owned()),
                measure: Some("7".to_owned()),
            })
        );
    }
}
