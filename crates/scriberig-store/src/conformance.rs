//! Defines the backend-neutral persistence conformance suite.

use std::collections::BTreeSet;

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RootName};

use crate::{
    MaintenanceStore, ProtectedRoots, Revision, RevisionStore, RootStore, Snapshot, SnapshotChange,
    SnapshotStore, StoreObservability,
};

/// Combined storage contract required by the shared conformance suite.
pub trait ConformanceStore: RevisionStore + SnapshotStore + RootStore {}

impl<T> ConformanceStore for T where T: RevisionStore + SnapshotStore + RootStore {}

/// Combined contract required by the complete persistence conformance suite.
pub trait FullConformanceStore: ConformanceStore + MaintenanceStore + StoreObservability {}

impl<T> FullConformanceStore for T where T: ConformanceStore + MaintenanceStore + StoreObservability {}

/// Deterministic identities produced by the shared branching fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConformanceFixture {
    /// Revision for entity A in the base history.
    pub a1: scriberig_core::RevisionId,
    /// Revision for entity A in branch S2.
    pub a2: scriberig_core::RevisionId,
    /// Revision for entity B in the base history.
    pub b1: scriberig_core::RevisionId,
    /// Revision for entity B in branch S3.
    pub b2: scriberig_core::RevisionId,
    /// Base snapshot containing A and B.
    pub s1: scriberig_core::SnapshotId,
    /// Branch changing A.
    pub s2: scriberig_core::SnapshotId,
    /// Branch changing B.
    pub s3: scriberig_core::SnapshotId,
}

/// Loads the deterministic S1/S2/S3 history into a backend.
///
/// # Parameters
/// - `store`: Backend receiving the canonical fixture.
///
/// # Returns
/// The logical identities produced by the fixture.
///
/// # Errors
/// Returns a diagnostic when the backend rejects a fixture operation.
pub fn load_fixture<S: ConformanceStore>(store: &S) -> Result<ConformanceFixture, String> {
    let kind =
        ObjectType::new("persistence.conformance.item").map_err(|error| format!("{error:?}"))?;
    let a1 = store
        .put(
            Revision::new(EncodingVersion::V1, kind.clone(), vec![1])
                .map_err(|error| format!("{error:?}"))?,
        )
        .map_err(|error| format!("{error:?}"))?;
    let a2 = store
        .put(
            Revision::new(EncodingVersion::V1, kind.clone(), vec![2])
                .map_err(|error| format!("{error:?}"))?,
        )
        .map_err(|error| format!("{error:?}"))?;
    let b1 = store
        .put(
            Revision::new(EncodingVersion::V1, kind.clone(), vec![3])
                .map_err(|error| format!("{error:?}"))?,
        )
        .map_err(|error| format!("{error:?}"))?;
    let b2 = store
        .put(
            Revision::new(EncodingVersion::V1, kind, vec![4])
                .map_err(|error| format!("{error:?}"))?,
        )
        .map_err(|error| format!("{error:?}"))?;
    let base = Snapshot::empty().id();
    let entity_a = EntityId::new([1; 16]);
    let entity_b = EntityId::new([2; 16]);
    let s1 = store
        .apply_changes(
            base,
            &[
                SnapshotChange::Set(entity_a, a1),
                SnapshotChange::Set(entity_b, b1),
            ],
        )
        .map_err(|error| format!("{error:?}"))?;
    let s2 = store
        .apply_changes(s1, &[SnapshotChange::Set(entity_a, a2)])
        .map_err(|error| format!("{error:?}"))?;
    let s3 = store
        .apply_changes(s1, &[SnapshotChange::Set(entity_b, b2)])
        .map_err(|error| format!("{error:?}"))?;
    Ok(ConformanceFixture {
        a1,
        a2,
        b1,
        b2,
        s1,
        s2,
        s3,
    })
}

/// Runs core immutable storage behavior against any backend.
///
/// # Parameters
/// - `store`: Backend instance under test.
///
/// # Errors
/// Returns a diagnostic when a backend violates a shared semantic invariant.
pub fn run_core_conformance<S: ConformanceStore>(store: &S) -> Result<(), String> {
    let kind = ObjectType::new("conformance.item").map_err(|error| format!("{error:?}"))?;
    let revision = Revision::new(EncodingVersion::V1, kind, vec![1, 2, 3])
        .map_err(|error| format!("{error:?}"))?;
    let revision_id = store
        .put(revision.clone())
        .map_err(|error| format!("{error:?}"))?;
    if store.put(revision).map_err(|error| format!("{error:?}"))? != revision_id {
        return Err("duplicate revision changed identity".to_owned());
    }
    if store
        .get(revision_id)
        .map_err(|error| format!("{error:?}"))?
        .is_none()
    {
        return Err("stored revision was not readable".to_owned());
    }

    let base = Snapshot::empty().id();
    let first_entity = EntityId::new([1; 16]);
    let second_entity = EntityId::new([2; 16]);
    let first = store
        .apply_changes(base, &[SnapshotChange::Set(first_entity, revision_id)])
        .map_err(|error| format!("{error:?}"))?;
    if store
        .resolve(first, first_entity)
        .map_err(|error| format!("{error:?}"))?
        != Some(revision_id)
    {
        return Err("snapshot did not retain its entity".to_owned());
    }
    let branch = store
        .apply_changes(base, &[SnapshotChange::Set(second_entity, revision_id)])
        .map_err(|error| format!("{error:?}"))?;
    if first == branch
        || !store
            .iter(base)
            .map_err(|error| format!("{error:?}"))?
            .is_empty()
    {
        return Err("snapshot branch changed its base".to_owned());
    }

    let root = RootName::new("conformance").map_err(|error| format!("{error:?}"))?;
    store
        .compare_and_set_root(root.clone(), None, first)
        .map_err(|error| format!("{error:?}"))?;
    if store
        .compare_and_set_root(root.clone(), None, branch)
        .is_ok()
    {
        return Err("stale root update succeeded".to_owned());
    }
    if store.read_root(&root) != Some(first) {
        return Err("stale root update changed the root".to_owned());
    }
    Ok(())
}

/// Runs the complete backend-neutral persistence contract.
///
/// # Parameters
/// - `store`: Backend instance under test.
///
/// # Errors
/// Returns a diagnostic when a backend violates the shared contract.
pub fn run_full_conformance<S: FullConformanceStore>(store: &S) -> Result<(), String> {
    run_core_conformance(store)?;
    let fixture = load_fixture(store)?;
    let root = RootName::new("fixture").map_err(|error| format!("{error:?}"))?;
    store
        .compare_and_set_root(root.clone(), None, fixture.s2)
        .map_err(|error| format!("{error:?}"))?;
    let stale = store
        .compare_and_set_root(root.clone(), None, fixture.s3)
        .expect_err("fixture root update must be stale");
    if stale.category() != crate::StoreErrorCategory::StaleRoot {
        return Err("stale root category was not backend-neutral".to_owned());
    }
    if store
        .resolve(fixture.s1, EntityId::new([2; 16]))
        .map_err(|error| format!("{error:?}"))?
        != Some(fixture.b1)
    {
        return Err("historical fixture snapshot was not readable".to_owned());
    }
    let protected = ProtectedRoots {
        snapshots: BTreeSet::from([fixture.s2]),
    };
    let report = store
        .gc_dry_run(&protected)
        .map_err(|error| error.to_string())?;
    if report.is_corrupt()
        || !report.unreachable.snapshots.contains(&fixture.s3)
        || report.unreachable.snapshots.contains(&fixture.s2)
        || report.gc.candidates == 0
        || report.gc.deleted != 0
    {
        return Err("reachability report did not preserve the protected branch".to_owned());
    }
    let stats = store
        .stats_for(&protected)
        .map_err(|error| error.to_string())?;
    if stats.reachable_objects.is_none() || stats.unreachable_objects.is_none() {
        return Err("backend omitted available reachability statistics".to_owned());
    }
    if store
        .iter(fixture.s2)
        .map_err(|error| format!("{error:?}"))?
        .is_empty()
    {
        return Err("statistics inspection changed snapshot semantics".to_owned());
    }
    let swept = store
        .gc_sweep(&protected)
        .map_err(|error| error.to_string())?;
    if swept.is_corrupt() || swept.gc.deleted != swept.gc.candidates {
        return Err("garbage collection did not report its complete sweep".to_owned());
    }
    if store
        .iter(fixture.s2)
        .map_err(|error| format!("{error:?}"))?
        .is_empty()
        || store.iter(fixture.s3).is_ok()
    {
        return Err("garbage collection did not preserve only protected history".to_owned());
    }
    let maintenance_indexes =
        MaintenanceStore::rebuild_indexes(store).map_err(|error| error.to_string())?;
    let observability_indexes =
        StoreObservability::rebuild_indexes(store).map_err(|error| error.to_string())?;
    if !maintenance_indexes.consistent
        || !observability_indexes.consistent
        || maintenance_indexes != observability_indexes
    {
        return Err("index rebuild did not report a consistent index".to_owned());
    }
    let statuses = store.root_statuses().map_err(|error| error.to_string())?;
    if !statuses.iter().any(|status| status.name == root) {
        return Err("root observability omitted the fixture root".to_owned());
    }
    Ok(())
}
