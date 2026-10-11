//! Defines semantic graph selections and transport-neutral projection anchors.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Identifies one typed semantic traversal step.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
    /// Adapter-defined anchor that is not part of canonical identity.
    Opaque {
        /// Anchor kind.
        kind: String,
        /// Adapter-defined anchor value.
        value: String,
    },
}

/// Selects one entity in an exact snapshot and optional semantic context.
#[derive(Clone, Debug, Eq, PartialEq)]
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
