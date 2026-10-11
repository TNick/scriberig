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

/// A stable public reference used by a future semantic selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticReference(pub String);

/// The primary view shown for a navigation location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimaryView {
    /// The default object overview.
    Overview,
    /// Immutable semantic history.
    History,
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
    /// The value produced by the declaration.
    fn default() -> Self {
        Self {
            selection: None,
            primary_view: PrimaryView::Overview,
            context: ViewContext::default(),
            entity: None,
        }
    }
}
