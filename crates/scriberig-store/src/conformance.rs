//! Defines the backend-neutral persistence conformance suite.

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RootName};

use crate::{Revision, RevisionStore, RootStore, Snapshot, SnapshotChange, SnapshotStore};

/// Combined storage contract required by the shared conformance suite.
pub trait ConformanceStore: RevisionStore + SnapshotStore + RootStore {}

impl<T> ConformanceStore for T where T: RevisionStore + SnapshotStore + RootStore {}

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
