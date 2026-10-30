//! Records direct-node revision identity across a retry lifecycle.

use scriberig_core::{EntityId, RevisionId, RootName};
use scriberig_session::{Plan, PlanNodeStatus};
use scriberig_store::{RootStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Verifies one canonical status and records its exact root-selected revision.
///
/// # Parameters
/// - `store`: SQLite universal semantic store.
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity.
/// - `projection`: Plan projection whose direct status is expected.
/// - `node`: Stable `PlanNode` entity identity.
/// - `expected_status`: Canonical direct lifecycle state expected now.
/// - `revisions`: Output list receiving the exact immutable revision.
pub(super) fn observe_node_state(
    store: &SqliteStore,
    session: EntityId,
    plan: EntityId,
    projection: &Plan,
    node: EntityId,
    expected_status: PlanNodeStatus,
    revisions: &mut Vec<RevisionId>,
) {
    assert_eq!(
        projection
            .nodes
            .iter()
            .find(|candidate| candidate.id == node)
            .expect("node in canonical projection")
            .status,
        expected_status
    );
    let root = RootName::new(format!("scriberig-plan-{session}-{plan}")).expect("plan root");
    let snapshot = store.read_root(&root).expect("current plan snapshot");
    let revision = store
        .iter(snapshot)
        .expect("read current plan snapshot")
        .into_iter()
        .find_map(|(entity, revision)| (entity == node).then_some(revision))
        .expect("node revision selected by root");
    revisions.push(revision);
}
