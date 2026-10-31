//! Defines typed DXF line edit candidate transport contracts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests an exact-snapshot CAD drawing summary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadDrawingInspectRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic drawing identity.
    pub drawing: String,
}

/// Returns drawing metadata and its semantic layers.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadDrawingInspectResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot used for all records.
    pub snapshot: String,
    /// Stable semantic drawing identity.
    pub drawing: String,
    /// Authored or adapter-reported unit label.
    pub units: String,
    /// Optional coordinate reference label.
    pub coordinate_reference: Option<String>,
    /// Ordered semantic layers.
    pub layers: Vec<CadLayerSummary>,
    /// Number of supported imported entities.
    pub entity_count: usize,
    /// Whether all source entities were represented.
    pub complete: bool,
    /// Safe fidelity and import diagnostics.
    pub diagnostics: Vec<String>,
}

/// Describes one semantic CAD layer.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadLayerSummary {
    /// Stable semantic layer identity.
    pub id: String,
    /// Authored layer name.
    pub name: String,
    /// Whether the layer is visible.
    pub visible: bool,
    /// Whether ordinary edits should treat the layer as locked.
    pub locked: bool,
}

/// Requests one bounded page of drawing entities.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadEntitiesRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic drawing identity.
    pub drawing: String,
    /// Zero-based source-order offset.
    pub offset: usize,
    /// Requested page size, at most five hundred.
    pub limit: usize,
}

/// Requests bounded authored text matches in one exact-snapshot drawing.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadTextFindRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic drawing identity.
    pub drawing: String,
    /// Case-insensitive text substring.
    pub query: String,
    /// Zero-based match offset.
    pub offset: usize,
    /// Requested page size, at most one hundred.
    pub limit: usize,
}

/// Returns bounded authored CAD text matches with stable entity handles.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadTextFindResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot used by every match.
    pub snapshot: String,
    /// Stable semantic drawing identity.
    pub drawing: String,
    /// Matching semantic entities and authored text.
    pub matches: Vec<CadTextMatch>,
    /// Total matches before pagination.
    pub total: usize,
    /// Whether another page remains.
    pub has_more: bool,
}

/// Identifies one authored CAD annotation match.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadTextMatch {
    /// Stable semantic entity and its typed geometry.
    pub entity: CadEntitySummary,
    /// Authored annotation text that matched.
    pub text: String,
}

/// Requests entities intersecting a two-dimensional envelope in one CAD space.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadBoundsQueryRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic drawing identity.
    pub drawing: String,
    /// CAD coordinate space to query.
    pub space: CadDrawingSpace,
    /// Two-dimensional bounds in exact decimal text.
    pub bounds: CadBounds2D,
    /// Zero-based result offset.
    pub offset: usize,
    /// Requested result count, at most five hundred.
    pub limit: usize,
}

/// Defines a two-dimensional selection envelope in CAD drawing units.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadBounds2D {
    /// Minimum x ordinate as decimal text.
    pub min_x: String,
    /// Minimum y ordinate as decimal text.
    pub min_y: String,
    /// Maximum x ordinate as decimal text.
    pub max_x: String,
    /// Maximum y ordinate as decimal text.
    pub max_y: String,
}

/// Returns one bounded CAD bounds query result.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadBoundsQueryResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot used for the query.
    pub snapshot: String,
    /// Stable semantic drawing identity.
    pub drawing: String,
    /// CAD coordinate space queried.
    pub space: CadDrawingSpace,
    /// Exact decimal-text query envelope.
    pub bounds: CadBounds2D,
    /// Matching semantic entities in source order.
    pub entities: Vec<CadEntitySummary>,
    /// Total matching entities before pagination.
    pub total: usize,
    /// Whether another result page remains.
    pub has_more: bool,
    /// Number of queried-space entities with invalid or unsupported extents.
    pub unbounded_entities: usize,
    /// Whether every entity in the queried space had a calculable extent.
    pub complete: bool,
}

/// Describes one semantic drawing entity without parser-native fields.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadEntitySummary {
    /// Stable semantic entity identity.
    pub id: String,
    /// Stable semantic layer identity.
    pub layer: String,
    /// Model, paper, or named-space membership.
    pub space: CadDrawingSpace,
    /// Typed CAD geometry serialized in the stable domain representation.
    pub geometry: CadGeometry,
    /// Optional adapter-neutral style reference.
    pub style: Option<CadEntityStyleRef>,
}

/// Represents model, paper, or named CAD drawing space.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum CadDrawingSpace {
    /// Main model space.
    Model,
    /// Paper space identified by a semantic layout ID.
    Paper(String),
    /// Adapter-defined named space.
    Named(String),
}

/// Identifies an adapter-neutral CAD style reference.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadEntityStyleRef {
    /// Adapter-owned style key.
    pub key: String,
    /// Adapter responsible for resolving the style.
    pub adapter: String,
}

/// Represents one typed format-independent CAD geometry variant.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum CadGeometry {
    /// Straight segment between two exact coordinates.
    Line {
        /// Exact start coordinate.
        start: CadCoordinate,
        /// Exact end coordinate.
        end: CadCoordinate,
    },
    /// Ordered polyline vertices.
    Polyline {
        /// Vertices in authored order.
        vertices: Vec<CadCoordinate>,
        /// Whether the last vertex connects to the first.
        closed: bool,
    },
    /// Circular geometry.
    Circle {
        /// Circle center.
        center: CadCoordinate,
        /// Exact radius in drawing units.
        radius: String,
    },
    /// Circular arc with adapter-defined angle units.
    Arc {
        /// Arc center.
        center: CadCoordinate,
        /// Exact radius in drawing units.
        radius: String,
        /// Exact starting angle.
        start_angle: String,
        /// Exact ending angle.
        end_angle: String,
    },
    /// Text annotation.
    Text {
        /// Text insertion point.
        position: CadCoordinate,
        /// Authored annotation text.
        value: String,
        /// Exact text height in drawing units.
        height: String,
    },
    /// Dimension definition points and optional override text.
    Dimension {
        /// Ordered geometry definition points.
        definition_points: Vec<CadCoordinate>,
        /// Optional authored override text.
        text: Option<String>,
    },
    /// Reference to a semantic block definition.
    BlockReference {
        /// Stable semantic block identity.
        definition: String,
        /// Insertion point in drawing units.
        insertion: CadCoordinate,
        /// Exact x, y, and z scale factors.
        scale: [String; 3],
        /// Rotation in adapter-defined units.
        rotation: String,
    },
}

/// Returns one bounded CAD entity page.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadEntitiesResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot used for all records.
    pub snapshot: String,
    /// Stable semantic drawing identity.
    pub drawing: String,
    /// Bounded semantic entities.
    pub entities: Vec<CadEntitySummary>,
    /// Total entity count.
    pub total: usize,
    /// Whether more entities remain.
    pub has_more: bool,
}

/// Requests a candidate edit of one exact-snapshot CAD line.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadLineEditRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot that anchors the line edit.
    pub snapshot: String,
    /// Stable semantic line identity.
    pub entity: String,
    /// Exact decimal start coordinates.
    pub start: CadCoordinate,
    /// Exact decimal end coordinates.
    pub end: CadCoordinate,
}

/// Represents one exact decimal coordinate tuple.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadCoordinate {
    /// Exact decimal x ordinate.
    pub x: String,
    /// Exact decimal y ordinate.
    pub y: String,
    /// Exact decimal z ordinate.
    pub z: String,
}

/// Returns one reviewable candidate handle for a CAD line edit.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadLineEditResponse {
    /// Stable candidate identity.
    pub candidate: String,
    /// Project owning the DXF source.
    pub project: String,
    /// Exact base snapshot bound to the candidate.
    pub base_snapshot: String,
    /// Project-relative DXF path.
    pub path: String,
    /// Validation state at candidate creation.
    pub validation: CadCandidateValidation,
}

/// Requests moving one exact-snapshot CAD entity to an existing semantic layer.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadLayerChangeRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot anchoring the operation.
    pub snapshot: String,
    /// Stable semantic CAD entity identity.
    pub entity: String,
    /// Stable semantic identity of an existing layer in the same drawing.
    pub destination_layer: String,
}

/// Returns a reviewable candidate handle for a CAD layer change.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CadLayerChangeResponse {
    /// Stable candidate identity.
    pub candidate: String,
    /// Project owning the DXF source.
    pub project: String,
    /// Exact base snapshot bound to the candidate.
    pub base_snapshot: String,
    /// Project-relative DXF path.
    pub path: String,
    /// Validation state at candidate creation.
    pub validation: CadCandidateValidation,
}

/// States current validation evidence for a CAD candidate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CadCandidateValidation {
    /// Candidate has not yet been validated.
    Pending,
    /// Candidate passed its requested validation profile.
    Passed,
    /// Candidate validation failed with safe diagnostics.
    Failed {
        /// Safe validation messages.
        diagnostics: Vec<String>,
    },
}
