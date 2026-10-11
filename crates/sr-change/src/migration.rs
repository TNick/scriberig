//! Defines exact snapshot transitions and entity deltas.

use crate::operation::Operation;
use crate::provenance::Provenance;
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Describes one entity transition between immutable snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityDelta {
    /// Entity whose canonical revision changes.
    pub entity: EntityId,
    /// Revision in the base snapshot, if present.
    pub before: Option<RevisionId>,
    /// Revision in the result snapshot, if present.
    pub after: Option<RevisionId>,
}

impl EntityDelta {
    /// Creates a non-no-op entity delta.
    ///
    /// # Parameters
    /// - `entity`: Entity being changed.
    /// - `before`: Base revision, if present.
    /// - `after`: Result revision, if present.
    ///
    /// # Returns
    /// The validated delta.
    ///
    /// # Errors
    /// Returns `MigrationError::NoOpDelta` when both revisions are equal.
    pub fn new(
        entity: EntityId,
        before: Option<RevisionId>,
        after: Option<RevisionId>,
    ) -> Result<Self, MigrationError> {
        if before == after {
            return Err(MigrationError::NoOpDelta { entity });
        }
        Ok(Self {
            entity,
            before,
            after,
        })
    }

    /// Reports whether the entity was created.
    #[must_use]
    pub const fn is_created(&self) -> bool {
        self.before.is_none() && self.after.is_some()
    }

    /// Reports whether the entity was removed.
    #[must_use]
    pub const fn is_removed(&self) -> bool {
        self.before.is_some() && self.after.is_none()
    }

    /// Reports whether both snapshots contain different revisions.
    #[must_use]
    pub const fn is_modified(&self) -> bool {
        self.before.is_some() && self.after.is_some()
    }
}

/// Reports invalid migration construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationError {
    /// Delta contains no semantic change.
    NoOpDelta {
        /// Entity whose delta was a no-op.
        entity: EntityId,
    },
    /// Entity appears more than once.
    DuplicateEntityDelta {
        /// Entity repeated in the migration.
        entity: EntityId,
    },
    /// Canonical migration uses the same base and result snapshot.
    InvalidSnapshotTransition {
        /// Snapshot used as both base and result.
        snapshot: SnapshotId,
    },
    /// Operation payload is invalid for a migration.
    InvalidOperation {
        /// Machine-readable invalid operation reason.
        reason: String,
    },
}

/// Records one exact canonical snapshot transition.
#[derive(Clone, Debug, PartialEq)]
pub struct Migration {
    /// Exact base snapshot.
    pub base: SnapshotId,
    /// Exact resulting snapshot.
    pub result: SnapshotId,
    /// Deterministically ordered entity deltas.
    pub deltas: Vec<EntityDelta>,
    /// Semantic operations applied by the migration.
    pub operations: Vec<Operation>,
    /// Source and audit metadata.
    pub provenance: Provenance,
}

impl Migration {
    /// Constructs a validated canonical migration.
    ///
    /// # Parameters
    /// - `base`: Exact starting snapshot.
    /// - `result`: Exact resulting snapshot.
    /// - `deltas`: Entity transitions.
    /// - `operations`: Semantic operations.
    /// - `provenance`: Source metadata.
    ///
    /// # Returns
    /// A migration with entity deltas sorted by identity.
    ///
    /// # Errors
    /// Returns a typed error for no-op snapshots, duplicate entities, or invalid deltas.
    pub fn new(
        base: SnapshotId,
        result: SnapshotId,
        mut deltas: Vec<EntityDelta>,
        operations: Vec<Operation>,
        provenance: Provenance,
    ) -> Result<Self, MigrationError> {
        if base == result && !deltas.is_empty() {
            return Err(MigrationError::InvalidSnapshotTransition { snapshot: base });
        }
        deltas.sort_by_key(|delta| delta.entity);
        for pair in deltas.windows(2) {
            if pair[0].entity == pair[1].entity {
                return Err(MigrationError::DuplicateEntityDelta {
                    entity: pair[0].entity,
                });
            }
        }
        Ok(Self {
            base,
            result,
            deltas,
            operations,
            provenance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityDelta, Migration, MigrationError};
    use crate::provenance::{Provenance, SourceKind};
    use scriberig_core::{EntityId, RevisionId, SnapshotId};
    use std::collections::BTreeMap;

    /// Creates empty provenance for migration fixtures.
    ///
    /// # Parameters
    /// - `source`: Universal source category.
    ///
    /// # Returns
    /// Empty provenance using the requested source.
    fn provenance(source: SourceKind) -> Provenance {
        Provenance {
            source,
            actor: None,
            workspace: None,
            projection: None,
            validation: Vec::new(),
            parents: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Verifies create, remove, and modify classifications.
    #[test]
    fn delta_classification_is_explicit() {
        let entity = EntityId::new([1; 16]);
        let before = RevisionId::new([2; 32]);
        let after = RevisionId::new([3; 32]);
        assert!(
            EntityDelta::new(entity, None, Some(after))
                .expect("create")
                .is_created()
        );
        assert!(
            EntityDelta::new(entity, Some(before), None)
                .expect("remove")
                .is_removed()
        );
        assert!(
            EntityDelta::new(entity, Some(before), Some(after))
                .expect("modify")
                .is_modified()
        );
        assert!(matches!(
            EntityDelta::new(entity, Some(before), Some(before)),
            Err(MigrationError::NoOpDelta { .. })
        ));
    }

    /// Verifies migration snapshots and deterministic entity ordering.
    #[test]
    fn migration_validates_snapshots_and_orders_deltas() {
        let first = EntityId::new([1; 16]);
        let second = EntityId::new([2; 16]);
        let base = SnapshotId::new([3; 32]);
        let result = SnapshotId::new([4; 32]);
        let deltas = vec![
            EntityDelta::new(second, None, Some(RevisionId::new([6; 32]))).expect("delta"),
            EntityDelta::new(first, None, Some(RevisionId::new([5; 32]))).expect("delta"),
        ];
        let migration = Migration::new(
            base,
            result,
            deltas,
            Vec::new(),
            provenance(SourceKind::User),
        )
        .expect("migration");
        assert_eq!(migration.base, base);
        assert_eq!(migration.deltas[0].entity, first);
        assert!(matches!(
            Migration::new(
                base,
                base,
                migration.deltas.clone(),
                Vec::new(),
                provenance(SourceKind::System)
            ),
            Err(MigrationError::InvalidSnapshotTransition { .. })
        ));
    }

    /// Verifies every universal provenance source remains constructible.
    #[test]
    fn provenance_sources_are_universal() {
        let sources = [
            SourceKind::User,
            SourceKind::ExternalProjection,
            SourceKind::Agent,
            SourceKind::Cli,
            SourceKind::Api,
            SourceKind::WebEditor,
            SourceKind::Tui,
            SourceKind::Importer,
            SourceKind::Refactoring,
            SourceKind::Merge,
            SourceKind::Synchronization,
            SourceKind::System,
            SourceKind::Other("extension".to_owned()),
        ];
        assert_eq!(sources.len(), 13);
    }
}
