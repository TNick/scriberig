//! Defines compact, presentation-neutral inspection and navigation results.

use crate::{DetailLevel, InformationCost, TargetRef};
use scriberig_core::{ObjectType, RevisionId, SnapshotId};

/// Advertises a typed capability available from an inspected entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadCapability {
    /// Stable semantic capability identifier.
    pub id: String,
    /// Short description of the available capability.
    pub description: String,
}

/// Describes one compact entity reached through a collection or relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationEntry {
    /// Snapshot-relative or exact-revision semantic handle.
    pub target: TargetRef,
    /// Entity type when known.
    pub entity_type: Option<ObjectType>,
    /// Optional short label.
    pub label: Option<String>,
    /// Optional short description.
    pub description: Option<String>,
    /// Optional coarse metadata or count.
    pub metadata: Option<String>,
}

/// Returns orientation metadata for one semantic entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inspection {
    /// Entity target that was inspected.
    pub subject: TargetRef,
    /// Exact snapshot used by the operation, when applicable.
    pub snapshot: Option<SnapshotId>,
    /// Exact revision resolved by the operation, when known.
    pub revision: Option<RevisionId>,
    /// Semantic entity type, when known.
    pub entity_type: Option<ObjectType>,
    /// Optional short label or title.
    pub label: Option<String>,
    /// Optional short description.
    pub description: Option<String>,
    /// Important properties represented by stable names and summaries.
    pub properties: Vec<(String, String)>,
    /// Coarse counts or structural metadata.
    pub counts: Vec<(String, u64)>,
    /// Typed relation capabilities.
    pub relations: Vec<ReadCapability>,
    /// Typed collection capabilities.
    pub collections: Vec<ReadCapability>,
    /// Available projection/view capabilities.
    pub views: Vec<ReadCapability>,
    /// Available semantic operation capabilities.
    pub operations: Vec<ReadCapability>,
    /// Compact provenance handles.
    pub provenance: Vec<String>,
    /// Advisory information-cost estimate.
    pub information_cost: Option<InformationCost>,
    /// Detail level used to produce this result.
    pub detail: DetailLevel,
}
