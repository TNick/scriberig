//! Converts geospatial domain values to and from provider-neutral protocol views.

use scriberig_domain_geo::{
    Coordinate, CoordinateReferenceSystem, Feature, FieldType, GeoValue, Geometry, GeometryType,
};
use scriberig_protocol::{
    GeoCoordinate, GeoCrsView, GeoFeatureView, GeoFieldType, GeoFieldView, GeoGeometry,
    GeoGeometryType, GeoLayerView,
};

/// Converts one semantic feature into its public typed representation.
///
/// # Parameters
/// - `feature`: Provider-neutral exact-snapshot feature.
///
/// Public feature with string identities and typed attributes/geometry.
///
/// # Returns
/// A wire-safe feature view.
pub(super) fn map_feature(feature: Feature) -> GeoFeatureView {
    GeoFeatureView {
        id: feature.id.to_string(),
        native_id: feature.native_id,
        geometry: feature.geometry.map(map_geometry),
        attributes: feature
            .attributes
            .into_iter()
            .map(|(key, value)| (key, map_value(value)))
            .collect(),
    }
}

/// Converts one semantic geometry recursively.
///
/// # Parameters
/// - `geometry`: Provider-neutral geometry in the declared layer CRS.
///
/// Typed protocol geometry without provider-specific handles.
///
/// # Returns
/// The equivalent typed protocol geometry.
fn map_geometry(geometry: Geometry) -> GeoGeometry {
    match geometry {
        Geometry::Point(coordinate) => GeoGeometry::Point(map_coordinate(coordinate)),
        Geometry::LineString(coordinates) => {
            GeoGeometry::LineString(coordinates.into_iter().map(map_coordinate).collect())
        }
        Geometry::Polygon(rings) => GeoGeometry::Polygon(
            rings
                .into_iter()
                .map(|ring| ring.into_iter().map(map_coordinate).collect())
                .collect(),
        ),
        Geometry::Collection(values) => {
            GeoGeometry::Collection(values.into_iter().map(map_geometry).collect())
        }
    }
}

/// Converts a typed protocol geometry into provider-neutral domain geometry.
///
/// # Parameters
/// - `geometry`: Typed wire geometry in the layer CRS.
///
/// # Returns
/// Equivalent domain geometry without provider-specific objects.
pub(super) fn unmap_geometry(geometry: GeoGeometry) -> Geometry {
    match geometry {
        GeoGeometry::Point(coordinate) => Geometry::Point(unmap_coordinate(coordinate)),
        GeoGeometry::LineString(coordinates) => {
            Geometry::LineString(coordinates.into_iter().map(unmap_coordinate).collect())
        }
        GeoGeometry::Polygon(rings) => Geometry::Polygon(
            rings
                .into_iter()
                .map(|ring| ring.into_iter().map(unmap_coordinate).collect())
                .collect(),
        ),
        GeoGeometry::Collection(values) => {
            Geometry::Collection(values.into_iter().map(unmap_geometry).collect())
        }
    }
}

/// Converts exact wire coordinate ordinates into the domain coordinate value.
///
/// # Parameters
/// - `coordinate`: Exact protocol coordinate.
///
/// # Returns
/// Equivalent domain coordinate.
fn unmap_coordinate(coordinate: GeoCoordinate) -> Coordinate {
    Coordinate {
        x: coordinate.x,
        y: coordinate.y,
        z: coordinate.z,
        measure: coordinate.measure,
    }
}

/// Converts one exact semantic coordinate into a protocol value.
///
/// # Parameters
/// - `coordinate`: Exact decimal coordinate ordinates.
///
/// Coordinate with CRS-independent exact decimal representation.
///
/// # Returns
/// The exact coordinate DTO.
fn map_coordinate(coordinate: Coordinate) -> GeoCoordinate {
    GeoCoordinate {
        x: coordinate.x,
        y: coordinate.y,
        z: coordinate.z,
        measure: coordinate.measure,
    }
}

/// Converts one typed source attribute value.
///
/// # Parameters
/// - `value`: Exact provider-neutral attribute value.
///
/// Typed protocol attribute value.
///
/// # Returns
/// The equivalent typed protocol value.
fn map_value(value: GeoValue) -> scriberig_protocol::GeoValue {
    match value {
        GeoValue::Null => scriberig_protocol::GeoValue::Null,
        GeoValue::Text(value) => scriberig_protocol::GeoValue::Text(value),
        GeoValue::Number(value) => scriberig_protocol::GeoValue::Number(value),
        GeoValue::Boolean(value) => scriberig_protocol::GeoValue::Boolean(value),
        GeoValue::DateTime(value) => scriberig_protocol::GeoValue::DateTime(value),
    }
}

/// Converts one project layer summary into its provider-neutral protocol view.
///
/// # Parameters
/// - `summary`: Application catalog layer with exact schema/fidelity details.
///
/// # Returns
/// Typed layer metadata without table handles or native provider objects.
pub(super) fn map_layer(summary: scriberig_project::GeoPackageLayerSummary) -> GeoLayerView {
    let layer = summary.layer;
    GeoLayerView {
        id: layer.id.to_string(),
        dataset: layer.dataset.to_string(),
        name: layer.name,
        fields: layer
            .fields
            .into_iter()
            .map(|field| GeoFieldView {
                name: field.name,
                field_type: map_field_type(field.field_type),
                nullable: field.nullable,
            })
            .collect(),
        crs: layer.crs.map(map_crs),
        geometry_type: layer.geometry_type.map(map_geometry_type),
        feature_count: summary.feature_count,
        extent: summary.extent,
        editable: summary.editable,
        diagnostics: summary.diagnostics,
    }
}

/// Converts one domain attribute schema type into a stable wire enum.
///
/// # Parameters
/// - `field_type`: Provider-neutral domain field type.
///
/// # Returns
/// The equivalent protocol field type.
fn map_field_type(field_type: FieldType) -> GeoFieldType {
    match field_type {
        FieldType::Text => GeoFieldType::Text,
        FieldType::Number => GeoFieldType::Number,
        FieldType::Boolean => GeoFieldType::Boolean,
        FieldType::DateTime => GeoFieldType::DateTime,
        FieldType::Binary => GeoFieldType::Binary,
    }
}

/// Converts CRS authority and definition metadata to the protocol view.
///
/// # Parameters
/// - `crs`: Provider-neutral exact CRS metadata.
///
/// # Returns
/// CRS metadata without native coordinate-system objects.
pub(super) fn map_crs(crs: CoordinateReferenceSystem) -> GeoCrsView {
    GeoCrsView {
        authority: crs.authority,
        code: crs.code,
        definition: crs.definition,
    }
}

/// Converts one domain geometry kind to its stable wire enum.
///
/// # Parameters
/// - `geometry_type`: Provider-neutral declared geometry kind.
///
/// # Returns
/// The equivalent protocol geometry type.
fn map_geometry_type(geometry_type: GeometryType) -> GeoGeometryType {
    match geometry_type {
        GeometryType::Point => GeoGeometryType::Point,
        GeometryType::LineString => GeoGeometryType::LineString,
        GeometryType::Polygon => GeoGeometryType::Polygon,
        GeometryType::GeometryCollection => GeoGeometryType::GeometryCollection,
    }
}
