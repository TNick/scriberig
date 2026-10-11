//! Defines explicit change-history topology.

use crate::migration::EntityDelta;
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Identifies one committed migration independently of storage.
pub type MigrationId = String;

/// Records one immutable migration edge in the history DAG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationReference {
    /// Stable migration identity.
    pub id: MigrationId,
    /// Base snapshot for this transition.
    pub base: SnapshotId,
    /// Result snapshot for this transition.
    pub result: SnapshotId,
    /// Parent migrations, including multiple merge lines.
    pub parents: Vec<MigrationId>,
    /// Entity transitions carried by this migration.
    pub deltas: Vec<EntityDelta>,
    /// Optional workspace or branch provenance.
    pub workspace: Option<String>,
}

/// Describes one entity revision transition in history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageRecord {
    /// Entity whose semantic identity continues.
    pub entity: EntityId,
    /// Revision before the transition.
    pub before: Option<RevisionId>,
    /// Revision after the transition.
    pub after: Option<RevisionId>,
    /// Migration introducing the transition.
    pub migration: MigrationId,
    /// Snapshot where the resulting revision became visible.
    pub visible_in: SnapshotId,
}

/// Stores immutable migration references and their explicit topology.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HistoryGraph {
    migrations: BTreeMap<MigrationId, MigrationReference>,
}

impl HistoryGraph {
    /// Creates an empty history graph.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            migrations: BTreeMap::new(),
        }
    }

    /// Adds one migration reference without relying on insertion order.
    ///
    /// # Parameters
    /// - `migration`: Immutable migration reference.
    ///
    /// # Returns
    /// `true` when inserted, or `false` when the same ID already exists.
    pub fn insert(&mut self, migration: MigrationReference) -> bool {
        if self.migrations.contains_key(&migration.id) {
            return false;
        }
        self.migrations.insert(migration.id.clone(), migration);
        true
    }

    /// Returns one migration by stable identity.
    ///
    /// # Parameters
    /// - `id`: Stable migration identity.
    ///
    /// # Returns
    /// The matching migration, when retained.
    #[must_use]
    pub fn migration(&self, id: &str) -> Option<&MigrationReference> {
        self.migrations.get(id)
    }

    /// Returns all migration references in stable migration-ID order.
    ///
    /// # Returns
    /// Borrowed migration references suitable for summary lists.
    #[must_use]
    pub fn migrations(&self) -> Vec<&MigrationReference> {
        self.migrations.values().collect()
    }

    /// Finds migrations that introduced an exact revision.
    ///
    /// # Parameters
    /// - `revision`: Revision identity to locate.
    ///
    /// # Returns
    /// Matching migration references in deterministic ID order.
    #[must_use]
    pub fn introducing_revision(&self, revision: RevisionId) -> Vec<&MigrationReference> {
        self.migrations
            .values()
            .filter(|migration| {
                migration
                    .deltas
                    .iter()
                    .any(|delta| delta.after == Some(revision))
            })
            .collect()
    }

    /// Finds migrations touching an entity.
    ///
    /// # Parameters
    /// - `entity`: Entity identity to locate.
    ///
    /// # Returns
    /// Matching migration references in deterministic ID order.
    #[must_use]
    pub fn touching_entity(&self, entity: EntityId) -> Vec<&MigrationReference> {
        self.migrations
            .values()
            .filter(|migration| migration.deltas.iter().any(|delta| delta.entity == entity))
            .collect()
    }

    /// Finds migrations associated with one workspace or branch.
    ///
    /// # Parameters
    /// - `workspace`: Exact workspace provenance value.
    ///
    /// # Returns
    /// Matching migration references in deterministic ID order.
    #[must_use]
    pub fn in_workspace(&self, workspace: &str) -> Vec<&MigrationReference> {
        self.migrations
            .values()
            .filter(|migration| migration.workspace.as_deref() == Some(workspace))
            .collect()
    }

    /// Returns descendant snapshot identities from explicit topology.
    ///
    /// # Parameters
    /// - `ancestor`: Snapshot from which descendants are requested.
    ///
    /// # Returns
    /// Reachable result snapshots sorted by canonical identity.
    #[must_use]
    pub fn descendants(&self, ancestor: SnapshotId) -> Vec<SnapshotId> {
        self.migrations
            .values()
            .filter(|migration| {
                migration.result != ancestor && self.is_ancestor(ancestor, migration.result)
            })
            .map(|migration| migration.result)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Returns all snapshots reachable as ancestors of a snapshot.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot whose ancestors are requested.
    ///
    /// # Returns
    /// A deterministic set of ancestor snapshots.
    #[must_use]
    pub fn ancestors(&self, snapshot: SnapshotId) -> BTreeSet<SnapshotId> {
        let mut result = BTreeSet::new();
        let mut queue = VecDeque::from([snapshot]);
        while let Some(current) = queue.pop_front() {
            for migration in self
                .migrations
                .values()
                .filter(|item| item.result == current)
            {
                if result.insert(migration.base) {
                    queue.push_back(migration.base);
                }
                for parent in &migration.parents {
                    if let Some(parent_migration) = self.migration(parent) {
                        if result.insert(parent_migration.result) {
                            queue.push_back(parent_migration.result);
                        }
                    }
                }
            }
        }
        result
    }

    /// Reports whether one snapshot is an ancestor of another.
    ///
    /// # Parameters
    /// - `candidate`: Potential ancestor.
    /// - `descendant`: Snapshot being examined.
    ///
    /// # Returns
    /// `true` when the candidate is equal to or precedes the descendant.
    #[must_use]
    pub fn is_ancestor(&self, candidate: SnapshotId, descendant: SnapshotId) -> bool {
        candidate == descendant || self.ancestors(descendant).contains(&candidate)
    }

    /// Returns common ancestors in deterministic snapshot order.
    ///
    /// # Parameters
    /// - `left`: First snapshot.
    /// - `right`: Second snapshot.
    ///
    /// # Returns
    /// Common ancestors sorted by canonical snapshot identity.
    #[must_use]
    pub fn common_ancestors(&self, left: SnapshotId, right: SnapshotId) -> Vec<SnapshotId> {
        let left_ancestors = self.ancestors(left);
        let right_ancestors = self.ancestors(right);
        left_ancestors
            .intersection(&right_ancestors)
            .copied()
            .collect()
    }

    /// Returns every lineage record for an entity in deterministic migration order.
    ///
    /// # Parameters
    /// - `entity`: Entity whose lineage is requested.
    ///
    /// # Returns
    /// Lineage records in migration identity order.
    #[must_use]
    pub fn lineage(&self, entity: EntityId) -> Vec<LineageRecord> {
        self.migrations
            .values()
            .flat_map(|migration| {
                migration.deltas.iter().filter_map(|delta| {
                    (delta.entity == entity).then_some(LineageRecord {
                        entity,
                        before: delta.before,
                        after: delta.after,
                        migration: migration.id.clone(),
                        visible_in: migration.result,
                    })
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{HistoryGraph, MigrationReference};
    use crate::migration::EntityDelta;
    use scriberig_core::{EntityId, RevisionId, SnapshotId};

    /// Creates a deterministic snapshot fixture.
    ///
    /// # Parameters
    /// - `value`: Repeated fixture byte.
    ///
    /// # Returns
    /// A deterministic snapshot identifier.
    fn snapshot(value: u8) -> SnapshotId {
        SnapshotId::new([value; 32])
    }
    /// Creates a deterministic migration fixture.
    ///
    /// # Parameters
    /// - `id`: Migration identity.
    /// - `base`: Base snapshot fixture byte.
    /// - `result`: Result snapshot fixture byte.
    /// - `parents`: Parent migration identifiers.
    ///
    /// # Returns
    /// A migration reference fixture.
    fn migration(id: &str, base: u8, result: u8, parents: Vec<&str>) -> MigrationReference {
        MigrationReference {
            id: id.to_owned(),
            base: snapshot(base),
            result: snapshot(result),
            parents: parents.into_iter().map(str::to_owned).collect(),
            deltas: Vec::new(),
            workspace: None,
        }
    }

    /// Verifies linear, sibling, merge, and common ancestry operations.
    #[test]
    fn dag_queries_are_insertion_order_independent() {
        let mut graph = HistoryGraph::new();
        graph.insert(migration("merge", 4, 5, vec!["left", "right"]));
        graph.insert(migration("right", 1, 4, vec![]));
        graph.insert(migration("left", 1, 3, vec![]));
        graph.insert(migration("base", 0, 1, vec![]));
        assert!(graph.is_ancestor(snapshot(1), snapshot(5)));
        assert!(graph.is_ancestor(snapshot(3), snapshot(5)));
        assert_eq!(
            graph.common_ancestors(snapshot(3), snapshot(4)),
            vec![snapshot(0), snapshot(1)]
        );
    }

    /// Verifies entity removal preserves its historical before revision.
    #[test]
    fn lineage_preserves_creation_and_removal() {
        let entity = EntityId::from_bytes([1; 16]);
        let revision = RevisionId::new([2; 32]);
        let delta = EntityDelta::new(entity, Some(revision), None).expect("removal");
        let mut graph = HistoryGraph::new();
        let mut reference = migration("remove", 1, 2, vec![]);
        reference.deltas.push(delta);
        graph.insert(reference);
        let lineage = graph.lineage(entity);
        assert_eq!(lineage[0].before, Some(revision));
        assert_eq!(lineage[0].after, None);
    }
}
