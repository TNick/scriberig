//! Defines isolated speculative work.

use scriberig_core::{Provenance, RootName, SnapshotId};
use scriberig_store::{RootStore, SnapshotStore};

use crate::{PromotionResult, promote_checked};

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
    /// Creates an active workspace rooted at an exact snapshot.
    #[must_use]
    /// - `base`: Argument named `base`.
    /// - `target_root`: Argument named `target_root`.
    /// - `provenance`: Argument named `provenance`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn new(base: SnapshotId, target_root: Option<RootName>, provenance: Provenance) -> Self {
        Self {
            base,
            current: base,
            target_root,
            status: WorkspaceStatus::Active,
            provenance,
        }
    }

    /// Advances a workspace head without changing its base.
    ///
    /// # Parameters
    /// - `snapshot`: New descendant snapshot.
    pub fn advance(&mut self, snapshot: SnapshotId) {
        self.current = snapshot;
    }

    /// Marks work as ready for explicit promotion.
    pub fn mark_ready(&mut self) {
        if self.status == WorkspaceStatus::Active {
            self.status = WorkspaceStatus::Ready;
        }
    }

    /// Abandons work without changing any named root.
    pub fn abandon(&mut self) {
        self.status = WorkspaceStatus::Abandoned;
    }

    /// Promotes a ready workspace through optimistic root advancement.
    pub fn promote<S: RootStore + SnapshotStore>(
        &mut self,
        store: &S,
        expected: Option<SnapshotId>,
    ) -> PromotionResult {
        let Some(root) = self.target_root.clone() else {
            return PromotionResult::InvalidTarget;
        };
        if self.status != WorkspaceStatus::Ready {
            return PromotionResult::InvalidTarget;
        }
        let result = promote_checked(store, root, expected, self.current);
        if result == PromotionResult::Promoted {
            self.status = WorkspaceStatus::Promoted;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{Workspace, WorkspaceStatus};
    use crate::PromotionResult;
    use scriberig_core::{Provenance, ProvenanceSource, RootName, SnapshotId};
    use scriberig_store::RootStore;
    use scriberig_store_memory::MemoryStore;

    /// Implements `workspace`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn workspace() -> Workspace {
        Workspace::new(
            SnapshotId::new([1; 32]),
            Some(RootName::new("main").expect("root")),
            Provenance {
                source: ProvenanceSource::User,
                producer: None,
                producer_version: None,
                inputs: Vec::new(),
                profile: None,
                note: None,
            },
        )
    }

    /// Verifies workspace construction fixes the base and current snapshots.
    #[test]
    fn construction_uses_snapshot_references() {
        let workspace = workspace();
        assert_eq!(workspace.base, SnapshotId::new([1; 32]));
        assert_eq!(workspace.current, workspace.base);
        assert_eq!(workspace.status, WorkspaceStatus::Active);
    }

    /// Verifies abandoned work cannot be mistaken for promotable work.
    #[test]
    fn abandon_marks_workspace_terminal() {
        let mut workspace = workspace();
        workspace.abandon();
        assert_eq!(workspace.status, WorkspaceStatus::Abandoned);
    }

    /// Verifies ready work promotes and records its terminal lifecycle state.
    #[test]
    fn ready_workspace_promotes_optimistically() {
        let store = MemoryStore::default();
        let base = store.empty_snapshot();
        let root = RootName::new("main").expect("root");
        let mut workspace = Workspace::new(
            base,
            Some(root.clone()),
            Provenance {
                source: ProvenanceSource::User,
                producer: None,
                producer_version: None,
                inputs: Vec::new(),
                profile: None,
                note: None,
            },
        );
        workspace.mark_ready();
        assert_eq!(workspace.promote(&store, None), PromotionResult::Promoted);
        assert_eq!(workspace.status, WorkspaceStatus::Promoted);
        assert_eq!(store.read_root(&root), Some(base));
    }
}
