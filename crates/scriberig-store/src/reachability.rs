//! Defines explicit protected-root reachability reporting.

use std::collections::BTreeSet;

use scriberig_core::{RevisionId, SnapshotId};

/// Identifies a persistent snapshot-map node.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SnapshotNodeId([u8; 32]);

impl SnapshotNodeId {
    /// Creates a node identity from its content digest.
    ///
    /// - `bytes`: Content digest of the persistent node.
    ///
    /// The persistent node identity.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the node identity bytes.
    ///
    /// The content digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Explicit immutable roots protected from maintenance deletion.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProtectedRoots {
    /// Snapshot identities retained as canonical roots.
    pub snapshots: BTreeSet<SnapshotId>,
}

/// Reachable immutable storage identities discovered from protected roots.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReachableObjects {
    /// Reachable snapshots.
    pub snapshots: BTreeSet<SnapshotId>,
    /// Reachable revisions.
    pub revisions: BTreeSet<RevisionId>,
    /// Reachable persistent snapshot-map nodes.
    pub snapshot_nodes: BTreeSet<SnapshotNodeId>,
}

/// Reports immutable objects that are safe candidates for collection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnreachableObjects {
    /// Valid but unprotected snapshots.
    pub snapshots: BTreeSet<SnapshotId>,
    /// Valid but unprotected revisions.
    pub revisions: BTreeSet<RevisionId>,
    /// Valid but unprotected persistent snapshot-map nodes.
    pub snapshot_nodes: BTreeSet<SnapshotNodeId>,
}
