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
pub mod snapshot_relation;
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
pub use snapshot_relation::{SnapshotRelation, SnapshotRelationChange};
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
    /// - `revision`: Immutable revision to store.
    ///
    /// The canonical revision identifier.
    ///
    /// Returns the backend storage error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn put(&self, revision: Revision) -> Result<scriberig_core::RevisionId, StoreError> {
        (**self).put(revision)
    }

    /// Delegates immutable revision lookup to the shared backend.
    ///
    /// - `id`: Revision identifier to look up.
    ///
    /// The revision when present.
    ///
    /// Returns the backend storage error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get(&self, id: scriberig_core::RevisionId) -> Result<Option<Revision>, StoreError> {
        (**self).get(id)
    }

    /// Delegates immutable revision presence checks to the shared backend.
    ///
    /// - `id`: Revision identifier to test.
    ///
    /// Whether the revision exists.
    ///
    /// Returns the backend storage error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn contains(&self, id: scriberig_core::RevisionId) -> Result<bool, StoreError> {
        (**self).contains(id)
    }
}

/// Shares snapshot storage without changing the backend contract.
impl<T: SnapshotStore + ?Sized> SnapshotStore for Arc<T> {
    /// Delegates entity resolution to the shared backend.
    ///
    /// - `snapshot`: Snapshot in which to resolve the entity.
    /// - `entity`: Entity identifier to resolve.
    ///
    /// The revision visible for the entity, when present.
    ///
    /// Returns the backend snapshot error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn resolve(
        &self,
        snapshot: scriberig_core::SnapshotId,
        entity: scriberig_core::EntityId,
    ) -> Result<Option<scriberig_core::RevisionId>, SnapshotError> {
        (**self).resolve(snapshot, entity)
    }

    /// Delegates immutable snapshot application to the shared backend.
    ///
    /// - `base`: Snapshot receiving the changes.
    /// - `changes`: Immutable entity changes to apply.
    ///
    /// The resulting snapshot identifier.
    ///
    /// Returns the backend snapshot error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn apply_changes(
        &self,
        base: scriberig_core::SnapshotId,
        changes: &[SnapshotChange],
    ) -> Result<scriberig_core::SnapshotId, SnapshotError> {
        (**self).apply_changes(base, changes)
    }

    /// Delegates relation changes to the shared backend.
    ///
    /// - `base`: Existing immutable snapshot.
    /// - `changes`: Typed relation changes.
    ///
    /// # Returns
    /// The resulting snapshot identity.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn apply_relation_changes(
        &self,
        base: scriberig_core::SnapshotId,
        changes: &[SnapshotRelationChange],
    ) -> Result<scriberig_core::SnapshotId, SnapshotError> {
        (**self).apply_relation_changes(base, changes)
    }

    /// Delegates atomic entity and relation changes to the shared backend.
    ///
    /// - `base`: Existing immutable snapshot.
    /// - `entity_changes`: Entity-to-revision changes.
    /// - `relation_changes`: Typed relation changes.
    ///
    /// # Returns
    /// The resulting snapshot identity.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn apply_changes_with_relations(
        &self,
        base: scriberig_core::SnapshotId,
        entity_changes: &[SnapshotChange],
        relation_changes: &[SnapshotRelationChange],
    ) -> Result<scriberig_core::SnapshotId, SnapshotError> {
        (**self).apply_changes_with_relations(base, entity_changes, relation_changes)
    }

    /// Delegates relation reads to the shared backend.
    ///
    /// - `snapshot`: Exact immutable snapshot.
    ///
    /// # Returns
    /// Relations visible at that snapshot.
    ///
    /// # Errors
    /// Returns the backend snapshot error.
    fn relations(
        &self,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<Vec<SnapshotRelation>, SnapshotError> {
        (**self).relations(snapshot)
    }

    /// Delegates deterministic snapshot iteration to the shared backend.
    ///
    /// - `snapshot`: Snapshot to enumerate.
    ///
    /// Entity and revision pairs visible in the snapshot.
    ///
    /// Returns the backend snapshot error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn iter(
        &self,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<Vec<(scriberig_core::EntityId, scriberig_core::RevisionId)>, SnapshotError> {
        (**self).iter(snapshot)
    }

    /// Delegates snapshot diffing to the shared backend.
    ///
    /// - `from`: Earlier snapshot.
    /// - `to`: Later snapshot.
    ///
    /// Changes needed to transform `from` into `to`.
    ///
    /// Returns the backend snapshot error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn diff_snapshots(
        &self,
        from: scriberig_core::SnapshotId,
        to: scriberig_core::SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError> {
        (**self).diff_snapshots(from, to)
    }

    /// Delegates semantic revision comparison to the shared backend.
    ///
    /// - `from`: Starting immutable snapshot.
    /// - `to`: Target immutable snapshot.
    ///
    /// Exact entity revision transitions in deterministic order.
    ///
    /// Returns the backend snapshot error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn diff_snapshot_revisions(
        &self,
        from: scriberig_core::SnapshotId,
        to: scriberig_core::SnapshotId,
    ) -> Result<Vec<SnapshotDiff>, SnapshotError> {
        (**self).diff_snapshot_revisions(from, to)
    }

    /// Delegates snapshot parent lookup to the shared backend.
    ///
    /// - `snapshot`: Snapshot whose parents are requested.
    ///
    /// Direct parent snapshot identifiers.
    ///
    /// Returns the backend snapshot error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn parents(
        &self,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<Vec<scriberig_core::SnapshotId>, SnapshotError> {
        (**self).parents(snapshot)
    }

    /// Delegates immutable snapshot change provenance lookup.
    ///
    /// - `snapshot`: Exact result snapshot to query.
    ///
    /// The backend's retained source-change identity, when present.
    ///
    /// Returns the backend snapshot error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `name`: Named root to read.
    ///
    /// The root snapshot, when the name is bound.
    /// # Returns
    /// The operation result.
    fn read_root(&self, name: &scriberig_core::RootName) -> Option<scriberig_core::SnapshotId> {
        (**self).read_root(name)
    }

    /// Delegates compare-and-set root promotion to the shared backend.
    ///
    /// - `name`: Named root to update.
    /// - `expected`: Snapshot currently expected at the root.
    /// - `new`: Snapshot to promote when the expectation matches.
    ///
    /// Success when the root was promoted.
    ///
    /// Returns a root conflict or backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `name`: Named root to update.
    /// - `snapshot`: Snapshot to bind to the root.
    ///
    /// Success when the root was assigned.
    ///
    /// Returns the backend root error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_root(
        &self,
        name: scriberig_core::RootName,
        snapshot: scriberig_core::SnapshotId,
    ) -> Result<(), RootError> {
        (**self).set_root(name, snapshot)
    }
}
