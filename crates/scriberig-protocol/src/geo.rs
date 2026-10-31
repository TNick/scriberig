//! Defines provider-neutral, snapshot-bound geospatial feature page contracts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Requests one bounded feature page from an exact `GeoPackage` source snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoFeaturesRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact project-relative path from the persisted scan.
    pub path: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Zero-based feature offset.
    pub offset: usize,
    /// Maximum feature count, from one through one thousand.
    pub limit: usize,
}

/// Requests one bounded typed geospatial predicate query at an exact snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoFeatureQueryRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact project-relative `GeoPackage` path.
    pub path: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Attribute equality or explicit-CRS envelope predicate.
    pub predicate: GeoFeaturePredicate,
    /// Zero-based matching feature offset.
    pub offset: usize,
    /// Maximum matching feature count, from one through one thousand.
    pub limit: usize,
}

/// Selects features with exact attribute equality or an explicit CRS envelope.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GeoFeaturePredicate {
    /// Matches one typed attribute value exactly.
    AttributeEquals {
        /// Authored field name.
        field: String,
        /// Typed expected value.
        value: GeoValue,
    },
    /// Matches geometries whose envelope overlaps explicit bounds.
    EnvelopeOverlaps {
        /// Query bounds and their explicit coordinate reference system.
        bounds: GeoFeatureBounds,
    },
}

/// Describes an axis-aligned envelope and its explicit CRS.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoFeatureBounds {
    /// Minimum x/easting decimal text.
    pub min_x: String,
    /// Minimum y/northing decimal text.
    pub min_y: String,
    /// Maximum x/easting decimal text.
    pub max_x: String,
    /// Maximum y/northing decimal text.
    pub max_y: String,
    /// Explicit bounds CRS.
    pub crs: GeoCrsView,
}

/// Returns bounded query features and exact source evaluation provenance.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoFeatureQueryResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact snapshot queried.
    pub snapshot: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Matching semantic feature rows.
    pub features: Vec<GeoFeatureView>,
    /// Whether another matching page exists.
    pub has_more: bool,
    /// Exact source revision identity.
    pub source_revision: String,
    /// Digest of source package bytes as lowercase hexadecimal.
    pub source_digest: String,
    /// Adapter used to read and evaluate the source.
    pub provider: String,
    /// Layer CRS used for predicate checks.
    pub layer_crs: Option<GeoCrsView>,
    /// Evaluation strategy name.
    pub strategy: String,
    /// Coordinate transformation description, if one was applied.
    pub coordinate_transform: Option<String>,
    /// Safe execution diagnostics.
    pub diagnostics: Vec<String>,
}

/// Requests a typed feature geometry candidate against an exact source snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoGeometryEditRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact project-relative `GeoPackage` path.
    pub path: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Stable semantic feature identity.
    pub feature: String,
    /// Replacement geometry in the layer CRS.
    pub geometry: GeoGeometry,
}

/// Returns a durable candidate handle for one geospatial feature geometry edit.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoGeometryEditResponse {
    /// Stable shared project candidate identity.
    pub candidate: String,
    /// Stable project identity.
    pub project: String,
    /// Exact base snapshot identity.
    pub base_snapshot: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Stable semantic feature identity.
    pub feature: String,
    /// Exact project-relative source path.
    pub path: String,
}

/// Requests a typed feature attribute candidate against an exact source snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoAttributeEditRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact project-relative `GeoPackage` path.
    pub path: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Stable semantic feature identity.
    pub feature: String,
    /// Authored schema field name.
    pub field: String,
    /// Typed replacement field value.
    pub value: GeoValue,
}

/// Returns a durable candidate handle for one feature attribute operation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoAttributeEditResponse {
    /// Stable shared project candidate identity.
    pub candidate: String,
    /// Stable project identity.
    pub project: String,
    /// Exact base snapshot identity.
    pub base_snapshot: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Stable semantic feature identity.
    pub feature: String,
    /// Authored schema field name.
    pub field: String,
    /// Exact project-relative source path.
    pub path: String,
}

/// Requests a bounded `GeoPackage` dataset and layer catalog page.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoCatalogRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact project-relative source path from the persisted scan.
    pub path: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
    /// Zero-based layer offset.
    pub offset: usize,
    /// Maximum layer count, from one through two hundred.
    pub limit: usize,
}

/// Returns bounded dataset and layer metadata without feature rows.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoCatalogResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
    /// Stable semantic dataset identity.
    pub dataset: String,
    /// BLAKE3 digest of the exact source package bytes.
    pub source_digest: String,
    /// Ordered layer metadata page.
    pub layers: Vec<GeoLayerView>,
    /// Total layer count before pagination.
    pub total: usize,
    /// Whether another page is available.
    pub has_more: bool,
}

/// Describes one semantic layer schema and its bounded summary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoLayerView {
    /// Stable semantic layer identity.
    pub id: String,
    /// Stable owning dataset identity.
    pub dataset: String,
    /// Authored layer name.
    pub name: String,
    /// Ordered provider-neutral attribute schema.
    pub fields: Vec<GeoFieldView>,
    /// Explicit CRS metadata when present.
    pub crs: Option<GeoCrsView>,
    /// Declared geometry kind when available.
    pub geometry_type: Option<GeoGeometryType>,
    /// Source feature count without enumerating feature rows.
    pub feature_count: u64,
    /// Authored extent in exact decimal text, when available.
    pub extent: Option<[String; 4]>,
    /// Whether the current adapter can safely edit the layer.
    pub editable: bool,
    /// Safe adapter diagnostics.
    pub diagnostics: Vec<String>,
}

/// Describes one provider-neutral layer field.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoFieldView {
    /// Authored field name.
    pub name: String,
    /// Stable semantic scalar type.
    pub field_type: GeoFieldType,
    /// Whether null values are allowed.
    pub nullable: bool,
}

/// Represents the provider-neutral type of one geospatial attribute field.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeoFieldType {
    /// UTF-8 text.
    Text,
    /// Exact decimal numeric text.
    Number,
    /// Boolean scalar.
    Boolean,
    /// Date/time text in source representation.
    DateTime,
    /// Binary data outside ordinary feature attributes.
    Binary,
}

/// Describes explicit coordinate reference metadata without native objects.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoCrsView {
    /// Authority namespace such as EPSG when known.
    pub authority: Option<String>,
    /// Authority code when known.
    pub code: Option<String>,
    /// Exact source CRS definition when available.
    pub definition: Option<String>,
}

/// Represents the declared provider-neutral geometry kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeoGeometryType {
    /// Point geometry.
    Point,
    /// Ordered line string.
    LineString,
    /// Polygon rings.
    Polygon,
    /// Mixed geometry collection.
    GeometryCollection,
}

/// Returns a bounded provider-neutral feature page from one exact snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoFeaturesResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot used for the page.
    pub snapshot: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Ordered semantic feature rows.
    pub features: Vec<GeoFeatureView>,
    /// Whether another bounded page is available.
    pub has_more: bool,
}

/// Describes one provider-neutral semantic feature.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoFeatureView {
    /// Stable Scriberig feature identity.
    pub id: String,
    /// Native provider identifier retained only as source evidence.
    pub native_id: Option<String>,
    /// Typed geometry in the layer's declared coordinate reference system.
    pub geometry: Option<GeoGeometry>,
    /// Authored typed feature attributes.
    pub attributes: BTreeMap<String, GeoValue>,
}

/// Represents provider-neutral geometry without native library handles.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", content = "coordinates", rename_all = "snake_case")]
pub enum GeoGeometry {
    /// One point coordinate.
    Point(GeoCoordinate),
    /// Ordered line string coordinates.
    LineString(Vec<GeoCoordinate>),
    /// Ordered polygon rings, with the first ring as the shell.
    Polygon(Vec<Vec<GeoCoordinate>>),
    /// Nested geometry values.
    Collection(Vec<GeoGeometry>),
}

/// Represents exact coordinate ordinates in the layer CRS.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GeoCoordinate {
    /// Exact x/easting decimal text.
    pub x: String,
    /// Exact y/northing decimal text.
    pub y: String,
    /// Optional exact z ordinate.
    pub z: Option<String>,
    /// Optional exact measure ordinate.
    pub measure: Option<String>,
}

/// Represents one typed geospatial feature attribute.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum GeoValue {
    /// Null attribute value.
    Null,
    /// UTF-8 text value.
    Text(String),
    /// Exact numeric text.
    Number(String),
    /// Boolean value.
    Boolean(bool),
    /// Date or timestamp in source representation.
    DateTime(String),
}
