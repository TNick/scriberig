//! Provides deterministic semantic data for the local development server.

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RootName, SnapshotId};
use scriberig_store::{Revision, RootStore, Snapshot, SnapshotChange, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Identifies the entities exposed by the local development fixture.
#[derive(Clone, Debug)]
pub struct DevelopmentFixture {
    /// Stable root entity shown when the browser starts.
    pub root_entity: EntityId,
    /// Stable children available through the root collection.
    pub child_entities: Vec<EntityId>,
    /// Snapshot containing the fixture entities.
    pub snapshot: SnapshotId,
}

/// Creates or reuses the deterministic browser development fixture.
///
/// # Parameters
/// - `store`: SQLite store receiving the fixture when it is absent.
///
/// # Returns
/// Fixture identities used by the HTTP bootstrap response.
///
/// # Errors
/// Returns a storage error when the fixture cannot be created or inspected.
pub fn ensure_fixture(store: &SqliteStore) -> Result<DevelopmentFixture, String> {
    let root = RootName::new("main").map_err(|error| error.to_string())?;
    if let Some(snapshot) = store.read_root(&root) {
        let entities = store.iter(snapshot).map_err(|error| format!("{error:?}"))?;
        if let Some((root_entity, _)) = entities.first() {
            return Ok(DevelopmentFixture {
                root_entity: *root_entity,
                child_entities: entities.iter().skip(1).map(|(entity, _)| *entity).collect(),
                snapshot,
            });
        }
    }

    let entities = [
        EntityId::from_bytes([1; 16]),
        EntityId::from_bytes([2; 16]),
        EntityId::from_bytes([3; 16]),
    ];
    let revisions = entities
        .iter()
        .enumerate()
        .map(|(index, _entity)| {
            let object_type =
                ObjectType::new("scriberig.demo.entity").map_err(|error| error.to_string())?;
            let payload = format!("development-{}", index + 1).into_bytes();
            Revision::new(EncodingVersion::V1, object_type, payload)
                .map_err(|error| format!("{error:?}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let base = Snapshot::empty().id();
    let changes = revisions
        .iter()
        .zip(entities)
        .map(|(revision, entity)| SnapshotChange::Set(entity, revision.id()))
        .collect::<Vec<_>>();
    let snapshot = store
        .apply_revisions_and_promote(root, None, base, &revisions, &changes)
        .map_err(|error| format!("{error:?}"))?;
    Ok(DevelopmentFixture {
        root_entity: entities[0],
        child_entities: entities[1..].to_vec(),
        snapshot,
    })
}
