//! Defines exact geospatial identities, values, geometry, and layer metadata.

use std::collections::BTreeMap;

use scriberig_core::EntityId;
use serde::{Deserialize, Serialize};

/// Represents a geospatial dataset and its ordered layers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeoDataset {
    /// Stable dataset semantic identity.
    pub id: EntityId,
    /// Ordered semantic layer identities.
    pub layers: Vec<EntityId>,
}

/// Represents a layer schema and its bounded feature collection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeoLayer {
    /// Stable layer semantic identity.
    pub id: EntityId,
    /// Owning dataset identity.
    pub dataset: EntityId,
    /// Authored layer name.
    pub name: String,
    /// Ordered attribute schema.
    pub fields: Vec<Field>,
    /// Explicit coordinate reference system.
    pub crs: Option<CoordinateReferenceSystem>,
    /// Declared geometry type when available.
    pub geometry_type: Option<GeometryType>,
}

/// Represents one schema field.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Authored field name.
    pub name: String,
    /// Semantic scalar type.
    pub field_type: FieldType,
    /// Whether null values are allowed.
    pub nullable: bool,
}

/// Gives one geospatial field definition its own stable semantic revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeoFieldDefinition {
    /// Stable semantic field identity.
    pub id: EntityId,
    /// Owning semantic layer identity.
    pub layer: EntityId,
    /// Exact typed field definition from the source schema.
    pub definition: Field,
}

/// Represents a provider-neutral attribute type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// UTF-8 text.
    Text,
    /// Exact decimal numeric text.
    Number,
    /// Boolean scalar.
    Boolean,
    /// Date or timestamp text in source representation.
    DateTime,
    /// Binary content represented outside ordinary feature attributes.
    Binary,
}

/// Represents one independently addressable feature.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    /// Stable Scriberig semantic identity.
    pub id: EntityId,
    /// Owning layer identity.
    pub layer: EntityId,
    /// Native provider row/feature ID retained as external evidence.
    pub native_id: Option<String>,
    /// Exact semantic geometry when present.
    pub geometry: Option<Geometry>,
    /// Attribute values keyed by authored field name.
    pub attributes: BTreeMap<String, GeoValue>,
}

/// Represents a typed feature attribute value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GeoValue {
    /// Null attribute.
    Null,
    /// UTF-8 text attribute.
    Text(String),
    /// Exact decimal numeric text.
    Number(String),
    /// Boolean attribute.
    Boolean(bool),
    /// Date/time in source representation.
    DateTime(String),
}

/// Represents the declared geometry kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GeometryType {
    /// Point geometry.
    Point,
    /// Ordered line string.
    LineString,
    /// Polygon rings with first ring as shell.
    Polygon,
    /// Provider-neutral mixed geometry collection.
    GeometryCollection,
}

/// Represents canonical coordinate content without provider-native handles.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Geometry {
    /// One point.
    Point(Coordinate),
    /// Ordered line vertices.
    LineString(Vec<Coordinate>),
    /// Ordered polygon rings.
    Polygon(Vec<Vec<Coordinate>>),
    /// Nested geometry values.
    Collection(Vec<Geometry>),
}

/// Represents exact coordinate ordinates in a layer's explicit CRS context.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    /// Exact x/easting decimal text.
    pub x: String,
    /// Exact y/northing decimal text.
    pub y: String,
    /// Optional exact z ordinate.
    pub z: Option<String>,
    /// Optional exact measure ordinate.
    pub measure: Option<String>,
}

/// Represents a CRS identity and source definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CoordinateReferenceSystem {
    /// Authority namespace such as EPSG when known.
    pub authority: Option<String>,
    /// Authority code when known.
    pub code: Option<String>,
    /// Exact source CRS definition when available.
    pub definition: Option<String>,
}

/// Derives a stable feature identity independent of a provider's public model.
///
/// - `dataset`: Owning dataset semantic identity.
/// - `layer`: Owning layer semantic identity.
/// - `native_id`: Stable source feature ID used as reconciliation evidence.
///
/// Deterministic semantic feature identity for this dataset/layer/source key.
/// # Returns
/// Stable semantic feature identity.
#[must_use]
pub fn semantic_feature_id(dataset: EntityId, layer: EntityId, native_id: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-geo-feature-v1\0");
    hasher.update(dataset.as_bytes());
    hasher.update(layer.as_bytes());
    hasher.update(native_id.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Derives a stable field identity independent of source column ordering.
///
/// # Parameters
/// - `layer`: Owning semantic layer identity.
/// - `name`: Exact authored source field name.
///
/// # Returns
/// Deterministic semantic field identity for the layer and name.
#[must_use]
pub fn semantic_field_id(layer: EntityId, name: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-geo-field-v1\0");
    hasher.update(layer.as_bytes());
    hasher.update(name.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}
