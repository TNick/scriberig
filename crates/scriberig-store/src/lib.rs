//! Defines backend-independent immutable revision, snapshot, and root storage contracts.

pub mod collection;
pub mod conformance;
pub mod integrity;
pub mod maintenance;
pub mod reachability;
pub mod revision;
pub mod revision_store;
pub mod roots;
pub mod snapshot;
pub mod snapshot_store;
pub mod stats;

pub use collection::Collection;
pub use conformance::{
    ConformanceFixture, ConformanceStore, FullConformanceStore, load_fixture, run_core_conformance,
    run_full_conformance,
};
pub use integrity::{IntegrityIssue, IntegrityReport};
pub use maintenance::MaintenanceStore;
pub use reachability::{ProtectedRoots, ReachableObjects, SnapshotNodeId, UnreachableObjects};
pub use revision::Revision;
pub use revision_store::{RevisionStore, StoreError};
pub use roots::{CasOutcome, RootError, RootStore};
pub use snapshot::{
    Snapshot, SnapshotChange, SnapshotChangeProvenance, SnapshotDiff, SnapshotError,
};
pub use snapshot_store::SnapshotStore;
pub use stats::{
    GcReport, IndexStatus, RootStatus, StoreErrorCategory, StoreObservability, StoreOperationError,
    StoreStats,
};

use std::sync::Arc;

/// Shares revision storage without changing the backend contract.
impl<T: RevisionStore + ?Sized> RevisionStore for Arc<T> {
    /// Delegates immutable revision insertion to the shared backend.
    ///
    /// # Parameters
    /// - `revision`: Immutable revision to store.
    ///
    /// # Returns
    /// The canonical revision identifier.
    ///
    /// # Errors
    /// Returns the backend storage error.
    fn put(&self, revision: Revision) -> Result<scriberig_core::RevisionId, StoreError> {
        (**self).put(revision)
    }

    /// Delegates immutable revision lookup to the shared backend.
    ///
    /// # Parameters
    /// - `id`: Revision identifier to look up.
    ///
    /// # Returns
    /// The revision when present.
    ///
    /// # Errors
    /// Returns the backend storage error.
    fn get(&self, id: scriberig_core::RevisionId) -> Result<Option<Revision>, StoreError> {
        (**self).get(id)
    }

    /// Delegates immutable revision presence checks to the shared backend.
    ///
    /// # Parameters
    /// - `id`: Revision identifier to test.
    ///
    /// # Returns
    /// Whether the revision exists.
    ///
    /// # Errors
    /// Returns the backend storage error.
    fn contains(&self, id: scriberig_core::RevisionId) -> Result<bool, StoreError> {
        (**self).contains(id)
    }
}

/// Shares snapshot storage without changing the backend contract.
impl<T: SnapshotStore + ?Sized> SnapshotStore for Arc<T> {
    /// Delegates entity resolution to the shared backend.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot in which to resolve the entity.
    /// - `entity`: Entity identifier to resolve.
    ///
    /// # Returns
    /// The revision visible for the entity, when present.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn resolve(
        &self,
        snapshot: scriberig_core::SnapshotId,
        entity: scriberig_core::EntityId,
    ) -> Result<Option<scriberig_core::RevisionId>, SnapshotError> {
        (**self).resolve(snapshot, entity)
    }

    /// Delegates immutable snapshot application to the shared backend.
    ///
    /// # Parameters
    /// - `base`: Snapshot receiving the changes.
    /// - `changes`: Immutable entity changes to apply.
    ///
    /// # Returns
    /// The resulting snapshot identifier.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn apply_changes(
        &self,
        base: scriberig_core::SnapshotId,
        changes: &[SnapshotChange],
    ) -> Result<scriberig_core::SnapshotId, SnapshotError> {
        (**self).apply_changes(base, changes)
    }

    /// Delegates deterministic snapshot iteration to the shared backend.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot to enumerate.
    ///
    /// # Returns
    /// Entity and revision pairs visible in the snapshot.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn iter(
        &self,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<Vec<(scriberig_core::EntityId, scriberig_core::RevisionId)>, SnapshotError> {
        (**self).iter(snapshot)
    }

    /// Delegates snapshot diffing to the shared backend.
    ///
    /// # Parameters
    /// - `from`: Earlier snapshot.
    /// - `to`: Later snapshot.
    ///
    /// # Returns
    /// Changes needed to transform `from` into `to`.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn diff_snapshots(
        &self,
        from: scriberig_core::SnapshotId,
        to: scriberig_core::SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError> {
        (**self).diff_snapshots(from, to)
    }

    /// Delegates snapshot parent lookup to the shared backend.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot whose parents are requested.
    ///
    /// # Returns
    /// Direct parent snapshot identifiers.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn parents(
        &self,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<Vec<scriberig_core::SnapshotId>, SnapshotError> {
        (**self).parents(snapshot)
    }

    /// Delegates immutable snapshot change provenance lookup.
    ///
    /// # Parameters
    /// - `snapshot`: Exact result snapshot to query.
    ///
    /// # Returns
    /// The backend's retained source-change identity, when present.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn change_provenance(
        &self,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<Option<SnapshotChangeProvenance>, SnapshotError> {
        (**self).change_provenance(snapshot)
    }
}

/// Shares named-root storage without changing the backend contract.
impl<T: RootStore + ?Sized> RootStore for Arc<T> {
    /// Delegates named-root lookup to the shared backend.
    ///
    /// # Parameters
    /// - `name`: Named root to read.
    ///
    /// # Returns
    /// The root snapshot, when the name is bound.
    fn read_root(&self, name: &scriberig_core::RootName) -> Option<scriberig_core::SnapshotId> {
        (**self).read_root(name)
    }

    /// Delegates compare-and-set root promotion to the shared backend.
    ///
    /// # Parameters
    /// - `name`: Named root to update.
    /// - `expected`: Snapshot currently expected at the root.
    /// - `new`: Snapshot to promote when the expectation matches.
    ///
    /// # Returns
    /// Success when the root was promoted.
    ///
    /// # Errors
    /// Returns a root conflict or backend error.
    fn compare_and_set_root(
        &self,
        name: scriberig_core::RootName,
        expected: Option<scriberig_core::SnapshotId>,
        new: scriberig_core::SnapshotId,
    ) -> Result<(), RootError> {
        (**self).compare_and_set_root(name, expected, new)
    }

    /// Delegates unconditional root assignment to the shared backend.
    ///
    /// # Parameters
    /// - `name`: Named root to update.
    /// - `snapshot`: Snapshot to bind to the root.
    ///
    /// # Returns
    /// Success when the root was assigned.
    ///
    /// # Errors
    /// Returns the backend root error.
    fn set_root(
        &self,
        name: scriberig_core::RootName,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<(), RootError> {
        (**self).set_root(name, snapshot)
    }
}
