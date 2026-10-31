//! Encodes one proposed plan snapshot and derives its atomic root changes.

use scriberig_core::{EncodingVersion, SnapshotId};
use scriberig_session::{Plan, PlanSemanticNodeRef};
use scriberig_store::{Revision, Snapshot, SnapshotChange, SnapshotStore};

use crate::SqliteSessionStore;

/// Holds semantic revisions, changes, and indexes for one root promotion.
///
/// # Public members
/// None.
///
/// # Private members
/// - `revisions`: Immutable object revisions to insert before promotion.
/// - `changes`: Exact entity-to-revision changes for the successor snapshot.
/// - `indexed_nodes`: Revision-bound nodes for the normalized direct index.
/// - `successor_snapshot`: Snapshot produced by applying `changes`.
pub(super) struct PlanWriteSet {
    /// Immutable object revisions to insert before root promotion.
    pub(super) revisions: Vec<Revision>,
    /// Exact entity-to-revision changes for the successor snapshot.
    pub(super) changes: Vec<SnapshotChange>,
    /// Node references used by the normalized direct-node index.
    pub(super) indexed_nodes: Vec<PlanSemanticNodeRef>,
    /// Snapshot produced by applying `changes` to the current snapshot.
    pub(super) successor_snapshot: SnapshotId,
}

/// Encodes plan revisions and computes the exact successor root snapshot.
///
/// # Parameters
/// - `store`: SQLite repository containing the universal semantic store.
/// - `current_snapshot`: Exact base snapshot to compare-and-swap.
/// - `proposed`: Validated plan projection being promoted.
///
/// # Returns
/// The semantic write set and exact successor snapshot.
///
/// # Errors
/// Returns a persistence diagnostic when encoding or reading the base snapshot fails.
pub(super) fn build_plan_write_set(
    store: &SqliteSessionStore,
    current_snapshot: SnapshotId,
    proposed: &Plan,
) -> Result<PlanWriteSet, String> {
    let encoded = scriberig_session::PlanSemanticSnapshot::from_plan(proposed)
        .map_err(|error| format!("plan encoding failed: {error:?}"))?;
    let indexed_nodes = encoded
        .node_refs()
        .map_err(|error| format!("plan node index encode failed: {error:?}"))?;
    let old_entries = store
        .semantic_store
        .iter(current_snapshot)
        .map_err(|error| format!("plan snapshot read failed: {error:?}"))?;
    let new_entities = encoded
        .objects
        .iter()
        .map(|object| object.entity)
        .collect::<std::collections::BTreeSet<_>>();
    let mut revisions = Vec::with_capacity(encoded.objects.len());
    let mut changes = Vec::with_capacity(encoded.objects.len() + old_entries.len());
    for object in encoded.objects {
        let revision = Revision::from_parts(
            object.revision,
            EncodingVersion::V1,
            object.object_type,
            object.payload,
        )
        .map_err(|error| format!("invalid plan revision: {error:?}"))?;
        changes.push(SnapshotChange::Set(object.entity, revision.id()));
        revisions.push(revision);
    }
    for (entity, _) in &old_entries {
        if !new_entities.contains(entity) {
            changes.push(SnapshotChange::Remove(*entity));
        }
    }
    let successor_snapshot = Snapshot::from_entries(old_entries.iter().copied().collect())
        .apply(changes.iter().copied())
        .id();
    Ok(PlanWriteSet {
        revisions,
        changes,
        indexed_nodes,
        successor_snapshot,
    })
}
