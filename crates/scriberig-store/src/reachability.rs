//! Defines explicit protected-root reachability reporting.

use scriberig_core::{RevisionId, SnapshotId};
use std::collections::BTreeSet;

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
}

/// Reports immutable objects that are safe candidates for collection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnreachableObjects {
    /// Valid but unprotected snapshots.
    pub snapshots: BTreeSet<SnapshotId>,
    /// Valid but unprotected revisions.
    pub revisions: BTreeSet<RevisionId>,
}
