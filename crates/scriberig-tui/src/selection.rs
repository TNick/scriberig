//! Defines stable semantic selections and presentation view context.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Identifies an entity without conflating its conceptual and exact context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntitySelection {
    /// Conceptual entity identity.
    pub entity: EntityId,
    /// Exact revision, when one was resolved.
    pub revision: Option<RevisionId>,
    /// Snapshot context, when one was resolved or requested.
    pub snapshot: Option<SnapshotId>,
}

/// Identifies a selectable semantic row independently of its current position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticRowIdentity {
    /// Root or child row in the semantic explorer.
    Explorer {
        /// Semantic entity represented by the row.
        entity: EntityId,
        /// Exact revision represented by the row.
        revision: Option<RevisionId>,
        /// Exact snapshot represented by the row.
        snapshot: Option<SnapshotId>,
        /// Whether the row is the explorer root.
        root: bool,
    },
    /// Immutable migration in semantic history.
    History(String),
    /// Entity result in a snapshot-bound semantic search.
    Find(EntityId, SnapshotId),
    /// Entity transition in an explicitly bounded diff.
    Diff(EntityId, SnapshotId, SnapshotId),
    /// Shared application operation.
    Operation(scriberig_core::OperationId),
}

/// Identifies one durable plan or plan node independently of list position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanSelection {
    /// Durable plan identity.
    pub plan: EntityId,
    /// Durable node identity, or `None` when the plan header is selected.
    pub node: Option<EntityId>,
}

/// A stable public reference used by a future semantic selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticReference(pub String);

/// The primary view shown for a navigation location.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PrimaryView {
    /// Default recursive agent conversation surface.
    Conversation,
    /// The default object overview.
    Overview,
    /// Snapshot-bound semantic exploration.
    Explorer,
    /// Immutable semantic history.
    History,
    /// Bounded semantic discovery results.
    Find,
    /// Two-sided semantic diff.
    Diff,
    /// Read-only rendered projection.
    Projection,
    /// Candidate/change review.
    Changes,
    /// Session tree and agent interaction.
    Session,
    /// Long-running operation observation.
    Operations,
    /// Durable plan structure and its separate dependency graph.
    Plan,
}

/// Explicit context needed to interpret a presentation location.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ViewContext {
    /// Optional branch or snapshot label.
    pub branch_or_snapshot: Option<String>,
    /// Optional comparison target reference.
    pub comparison_target: Option<SemanticReference>,
}

/// A stable navigation location independent from cursor position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationLocation {
    /// Optional semantic selection.
    pub selection: Option<SemanticReference>,
    /// Primary view selected at this location.
    pub primary_view: PrimaryView,
    /// Explicit snapshot/branch/comparison context.
    pub context: ViewContext,
    /// Exact entity selection represented by this location.
    pub entity: Option<EntitySelection>,
}

impl Default for NavigationLocation {
    /// Creates a location with no semantic selection.
    ///
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self {
            selection: None,
            primary_view: PrimaryView::Conversation,
            context: ViewContext::default(),
            entity: None,
        }
    }
}
