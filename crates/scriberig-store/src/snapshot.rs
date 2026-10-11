//! Defines immutable backend-independent snapshot mappings.

use std::collections::BTreeMap;
use std::sync::Arc;

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Describes one storage-level snapshot update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotChange {
    /// Associates an entity with a revision.
    Set(EntityId, RevisionId),
    /// Removes an entity association.
    Remove(EntityId),
}

/// Reports an invalid snapshot operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    /// The requested snapshot is not known by the backend.
    UnknownSnapshot(SnapshotId),
}

/// An immutable entity-to-revision mapping with a content-derived identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    id: SnapshotId,
    entries: Arc<BTreeMap<EntityId, RevisionId>>,
}

impl Snapshot {
    /// # Parameters
    /// - `entries`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Creates a canonical snapshot from an entity mapping.
    #[must_use]
    pub fn from_entries(entries: BTreeMap<EntityId, RevisionId>) -> Self {
        let mut bytes = Vec::with_capacity(entries.len() * 48);
        for (entity, revision) in &entries {
            bytes.extend_from_slice(entity.as_bytes());
            bytes.extend_from_slice(revision.as_bytes());
        }
        let id = SnapshotId::from_bytes(*blake3::hash(&bytes).as_bytes());
        Self {
            id,
            entries: Arc::new(entries),
        }
    }

    ///
    /// # Returns
    /// The requested operation result.
    /// Returns this snapshot's content-derived identity.
    #[must_use]
    pub const fn id(&self) -> SnapshotId {
        self.id
    }

    /// # Parameters
    /// - `entity`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Resolves an entity in this immutable snapshot.
    #[must_use]
    pub fn resolve(&self, entity: EntityId) -> Option<RevisionId> {
        self.entries.get(&entity).copied()
    }

    ///
    /// # Returns
    /// The requested operation result.
    /// Iterates entity mappings in deterministic entity order.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, RevisionId)> + '_ {
        self.entries
            .iter()
            .map(|(entity, revision)| (*entity, *revision))
    }

    /// # Parameters
    /// - `changes`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Applies storage changes while sharing the unchanged map structure.
    #[must_use]
    pub fn apply(&self, changes: impl IntoIterator<Item = SnapshotChange>) -> Self {
        let mut entries = (*self.entries).clone();
        for change in changes {
            match change {
                SnapshotChange::Set(entity, revision) => {
                    entries.insert(entity, revision);
                }
                SnapshotChange::Remove(entity) => {
                    entries.remove(&entity);
                }
            }
        }
        Self::from_entries(entries)
    }

    /// # Parameters
    /// - `other`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Computes exact mapping changes from this snapshot to another.
    #[must_use]
    pub fn diff(&self, other: &Self) -> Vec<SnapshotChange> {
        let mut changes = Vec::new();
        for (entity, revision) in other.iter() {
            if self.resolve(entity) != Some(revision) {
                changes.push(SnapshotChange::Set(entity, revision));
            }
        }
        for (entity, _) in self.iter() {
            if other.resolve(entity).is_none() {
                changes.push(SnapshotChange::Remove(entity));
            }
        }
        changes.sort_by_key(|change| match change {
            SnapshotChange::Set(entity, _) | SnapshotChange::Remove(entity) => *entity,
        });
        changes
    }
}

/// Defines the backend-neutral snapshot operations.
pub trait SnapshotStore {
    /// # Parameters
    /// - `snapshot`: Relevant operation input.
    /// - `entity`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Resolves an entity in a stored snapshot.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unknown.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, SnapshotError>;
    /// # Parameters
    /// - `base`: Relevant operation input.
    /// - `changes`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Applies storage changes and returns the resulting snapshot.
    ///
    /// # Errors
    /// Returns an error when the base snapshot is unknown.
    fn apply_changes(
        &self,
        base: SnapshotId,
        changes: &[SnapshotChange],
    ) -> Result<SnapshotId, SnapshotError>;
    /// # Parameters
    /// - `snapshot`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Iterates a stored snapshot in deterministic order.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unknown.
    #[allow(clippy::iter_not_returning_iterator)]
    fn iter(&self, snapshot: SnapshotId) -> Result<Vec<(EntityId, RevisionId)>, SnapshotError>;
    /// # Parameters
    /// - `from`: Relevant operation input.
    /// - `to`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Computes exact mapping changes between two stored snapshots.
    ///
    /// # Errors
    /// Returns an error when either snapshot is unknown.
    fn diff_snapshots(
        &self,
        from: SnapshotId,
        to: SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError>;
}
