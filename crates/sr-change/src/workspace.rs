//! Defines isolated speculative work.

use scriberig_core::{Provenance, RootName, SnapshotId};

/// Lifecycle of an isolated workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceStatus {
    /// Work is active.
    Active,
    /// Work is ready for promotion.
    Ready,
    /// Work was promoted.
    Promoted,
    /// Work was abandoned.
    Abandoned,
    /// Validation failed.
    FailedValidation,
}

/// A workspace references immutable snapshots rather than copying them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workspace {
    /// Fixed base snapshot.
    pub base: SnapshotId,
    /// Current descendant snapshot.
    pub current: SnapshotId,
    /// Optional root targeted by promotion.
    pub target_root: Option<RootName>,
    /// Current lifecycle status.
    pub status: WorkspaceStatus,
    /// Origin metadata.
    pub provenance: Provenance,
}

impl Workspace {
    /// Advances a workspace head without changing its base.
    ///
    /// # Parameters
    /// - `snapshot`: New descendant snapshot.
    pub fn advance(&mut self, snapshot: SnapshotId) {
        self.current = snapshot;
    }
}
