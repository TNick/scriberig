//! Defines semantic graph selections and transport-neutral projection anchors.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Identifies one typed semantic traversal step.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RelationStep {
    /// Follows a typed relation.
    Relation {
        /// Stable semantic relation type.
        relation_type: String,
    },
    /// Follows a typed collection.
    Collection {
        /// Stable semantic collection type.
        collection_type: String,
    },
}

/// Maps a semantic entity to an optional source projection location.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ProjectionAnchor {
    /// Character or byte range in a text projection.
    TextRange {
        /// Inclusive range start.
        start: u64,
        /// Exclusive range end.
        end: u64,
    },
    /// Region on a paginated document projection.
    PageRegion {
        /// One-based page number.
        page: u32,
        /// Horizontal coordinate.
        x: u32,
        /// Vertical coordinate.
        y: u32,
        /// Region width.
        width: u32,
        /// Region height.
        height: u32,
    },
    /// One cell in a spreadsheet projection.
    SpreadsheetCell {
        /// Stable semantic worksheet identity.
        worksheet: EntityId,
        /// One-based row coordinate.
        row: u32,
        /// One-based column coordinate.
        column: u32,
    },
    /// Inclusive cell range in a spreadsheet projection.
    SpreadsheetRange {
        /// Stable semantic worksheet identity.
        worksheet: EntityId,
        /// First one-based row.
        first_row: u32,
        /// First one-based column.
        first_column: u32,
        /// Last one-based row.
        last_row: u32,
        /// Last one-based column.
        last_column: u32,
    },
    /// CRS-aware geometry bounds in exact adapter-neutral decimal text.
    GeometryBounds {
        /// Coordinate reference system identifier.
        crs: String,
        /// Minimum x coordinate as exact decimal text.
        min_x: String,
        /// Minimum y coordinate as exact decimal text.
        min_y: String,
        /// Maximum x coordinate as exact decimal text.
        max_x: String,
        /// Maximum y coordinate as exact decimal text.
        max_y: String,
    },
    /// Projection-space bounds for one selected CAD entity.
    CadBounds {
        /// Stable semantic drawing identity.
        drawing_id: String,
        /// Model, paper, or named CAD space identity.
        space: String,
        /// Minimum x coordinate as exact drawing-unit text.
        min_x: String,
        /// Minimum y coordinate as exact drawing-unit text.
        min_y: String,
        /// Maximum x coordinate as exact drawing-unit text.
        max_x: String,
        /// Maximum y coordinate as exact drawing-unit text.
        max_y: String,
    },
    /// Adapter-defined anchor that is not part of canonical identity.
    Opaque {
        /// Adapter owning the projection anchor schema.
        #[serde(default)]
        adapter: String,
        /// Anchor kind.
        kind: String,
        /// Adapter-defined anchor value.
        value: String,
    },
}

/// Selects one entity in an exact snapshot and optional semantic context.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Selection {
    /// Exact snapshot containing the selection.
    pub snapshot: SnapshotId,
    /// Selected entity identity.
    pub entity: EntityId,
    /// Exact selected revision when resolved.
    pub revision: Option<RevisionId>,
    /// Semantic relation/collection path leading to the entity.
    pub relation_path: Vec<RelationStep>,
    /// Optional convenience mapping into a projection.
    pub projection_anchor: Option<ProjectionAnchor>,
}
