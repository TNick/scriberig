//! Defines semantic CAD structures and exact decimal coordinate values.

use scriberig_core::EntityId;
use serde::{Deserialize, Serialize};

/// Represents one drawing and its ordered semantic content.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    /// Stable drawing semantic identity.
    pub id: EntityId,
    /// Authored linear unit label or adapter-specific unit code.
    pub units: String,
    /// Optional coordinate reference system identifier.
    pub coordinate_reference: Option<String>,
    /// Drawing layers in authored order.
    pub layers: Vec<Layer>,
    /// Drawing entities in source order when the format preserves it.
    pub entities: Vec<DrawingEntity>,
    /// Block definitions separate from their references.
    pub blocks: Vec<BlockDefinition>,
    /// Model/paper layouts.
    pub layouts: Vec<Layout>,
    /// Explicit external drawing references.
    pub external_references: Vec<ExternalReference>,
}

/// Represents a semantic drawing layer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    /// Stable layer semantic identity.
    pub id: EntityId,
    /// Authored layer name.
    pub name: String,
    /// Whether entities on this layer are visible.
    pub visible: bool,
    /// Whether entities on this layer are protected from ordinary edits.
    pub locked: bool,
}

/// Represents one independently addressable entity and its layer relation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DrawingEntity {
    /// Stable Scriberig semantic identity.
    pub id: EntityId,
    /// Semantic layer membership.
    pub layer: EntityId,
    /// Model or paper space membership.
    pub space: DrawingSpace,
    /// Typed geometry or annotation.
    pub geometry: CadEntity,
    /// Optional adapter-neutral style reference.
    pub style: Option<EntityStyleRef>,
}

/// Represents a style identity independent of geometry semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntityStyleRef {
    /// Adapter-owned style key.
    pub key: String,
    /// Adapter responsible for resolving the style.
    pub adapter: String,
}

/// Represents the drawing space that owns one entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DrawingSpace {
    /// Main model space.
    Model,
    /// Paper space with a stable layout identity.
    Paper(EntityId),
    /// Adapter-defined named space.
    Named(String),
}

/// Represents supported geometry and annotation types without group-code bags.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CadEntity {
    /// Straight segment between exact coordinates.
    Line {
        /// Start point in drawing coordinates.
        start: Coordinate,
        /// End point in drawing coordinates.
        end: Coordinate,
    },
    /// Ordered sequence of polyline vertices.
    Polyline {
        /// Vertices in authored order.
        vertices: Vec<Coordinate>,
        /// Whether the last vertex connects to the first.
        closed: bool,
    },
    /// Circular geometry.
    Circle {
        /// Center point in drawing coordinates.
        center: Coordinate,
        /// Exact radius in drawing units.
        radius: String,
    },
    /// Circular arc with start/end angles in documented adapter units.
    Arc {
        /// Center point in drawing coordinates.
        center: Coordinate,
        /// Exact radius in drawing units.
        radius: String,
        /// Exact starting angle in adapter-defined units.
        start_angle: String,
        /// Exact ending angle in adapter-defined units.
        end_angle: String,
    },
    /// Authored text annotation.
    Text {
        /// Text insertion point in drawing coordinates.
        position: Coordinate,
        /// Authored annotation text.
        value: String,
        /// Exact text height in drawing units.
        height: String,
    },
    /// Dimension referencing measured geometry.
    Dimension {
        /// Ordered geometry definition points.
        definition_points: Vec<Coordinate>,
        /// Authored dimension override text when present.
        text: Option<String>,
    },
    /// Reference to a separately modeled block definition.
    BlockReference(BlockReference),
}

/// Represents one exact coordinate tuple in drawing units.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    /// Exact decimal x coordinate.
    pub x: String,
    /// Exact decimal y coordinate.
    pub y: String,
    /// Exact decimal z coordinate.
    pub z: String,
}

impl Coordinate {
    /// Creates a coordinate from exact decimal text.
    ///
    /// - `x`: Exact x coordinate in drawing units.
    /// - `y`: Exact y coordinate in drawing units.
    /// - `z`: Exact z coordinate in drawing units.
    ///
    /// # Returns
    /// Coordinate preserving the supplied decimal spellings.
    #[must_use]
    pub fn new(x: impl Into<String>, y: impl Into<String>, z: impl Into<String>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
        }
    }
}

/// Represents a reusable block definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockDefinition {
    /// Stable block semantic identity.
    pub id: EntityId,
    /// Authored block name.
    pub name: String,
    /// Entity identities that form the block contents.
    pub entities: Vec<EntityId>,
}

/// Represents an insertion of a block definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockReference {
    /// Referenced block definition identity.
    pub definition: EntityId,
    /// Insertion point in drawing units.
    pub insertion: Coordinate,
    /// X/Y/Z scale factors as exact decimal text.
    pub scale: [String; 3],
    /// Rotation angle in adapter-defined units.
    pub rotation: String,
}

/// Represents a named paper-space layout.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    /// Stable layout semantic identity.
    pub id: EntityId,
    /// Authored layout name.
    pub name: String,
    /// Viewports associated with this layout.
    pub viewports: Vec<Viewport>,
}

/// Represents a viewport into model space.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    /// Stable viewport semantic identity.
    pub id: EntityId,
    /// View center in model-space drawing units.
    pub center: Coordinate,
    /// View height in drawing units.
    pub height: String,
}

/// Represents an external drawing reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExternalReference {
    /// Stable relation identity.
    pub id: EntityId,
    /// Authored external target path or URI.
    pub target: String,
    /// Optional referenced semantic drawing identity.
    pub drawing: Option<EntityId>,
}
