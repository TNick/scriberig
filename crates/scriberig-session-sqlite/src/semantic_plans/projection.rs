//! Loads compact plans from exact semantic snapshots without detail payloads.

use rusqlite::{Connection, params};
use scriberig_core::{EntityId, RevisionId, SnapshotId, TypeId};
use scriberig_session::Plan;
use scriberig_session::schema::SessionEntityType;
use scriberig_store::{RootStore, SnapshotStore};

use crate::SqliteSessionStore;

impl SqliteSessionStore {
    /// Loads compact current plans without gate, evidence, or history payloads.
    ///
    /// Historical snapshots include their detail records because callers of
    /// the historical plan API need the complete semantic projection.
    ///
    /// # Parameters
    /// - `self`: SQLite session store containing semantic objects.
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable semantic plan identity.
    /// - `snapshot`: Exact historical snapshot, or `None` for current state.
    ///
    /// # Returns
    /// The exact compact projection, or `None` when outside plan history.
    ///
    /// # Errors
    /// Returns a diagnostic when snapshot resolution or semantic decoding fails.
    pub(crate) fn load_plan_projection(
        &self,
        session: EntityId,
        plan: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<Option<Plan>, String> {
        let root = super::plan_root(session, plan)?;
        let Some(current) = self.semantic_store.read_root(&root) else {
            return Ok(None);
        };
        let resolved = if let Some(snapshot) = snapshot {
            let ancestors = self
                .semantic_store
                .ancestors(current)
                .map_err(|error| format!("plan history read failed: {error:?}"))?;
            if snapshot != current && !ancestors.contains(&snapshot) {
                return Ok(None);
            }
            snapshot
        } else {
            current
        };
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let objects = read_projection_objects(&connection, resolved, snapshot.is_some())?;
        let semantic = scriberig_session::PlanSemanticSnapshot {
            plan,
            snapshot: Some(resolved),
            objects,
        };
        semantic
            .into_plan(Vec::new())
            .map(Some)
            .map_err(|error| format!("compact plan semantic decode failed: {error:?}"))
    }
}

/// Reads typed semantic revisions, omitting current-only detail payloads.
///
/// # Parameters
/// - `connection`: SQLite connection sharing the semantic object database.
/// - `snapshot`: Exact immutable snapshot to project.
/// - `include_details`: Whether historical gates and evidence are required.
///
/// # Returns
/// Typed object revisions required to assemble the requested projection.
///
/// # Errors
/// Returns a diagnostic when object identity, type, or payload columns are
/// malformed.
fn read_projection_objects(
    connection: &Connection,
    snapshot: SnapshotId,
    include_details: bool,
) -> Result<Vec<scriberig_session::PlanSemanticRevision>, String> {
    let records = if include_details {
        let mut statement = connection
            .prepare(
                "SELECT entries.entity, entries.revision, revisions.object_type,
                        revisions.payload
                 FROM snapshot_entries AS entries
                 JOIN revisions ON revisions.id=entries.revision
                 WHERE entries.snapshot_id=?1 AND revisions.object_type!=?2
                 ORDER BY entries.entity",
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map(
                params![
                    snapshot.as_bytes().as_slice(),
                    SessionEntityType::PlanRevision.as_str()
                ],
                raw_plan_revision,
            )
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    } else {
        let mut statement = connection
            .prepare(
                "SELECT entries.entity, entries.revision, revisions.object_type,
                        revisions.payload
                 FROM snapshot_entries AS entries
                 JOIN revisions ON revisions.id=entries.revision
                 WHERE entries.snapshot_id=?1
                   AND revisions.object_type NOT IN (?2, ?3, ?4)
                 ORDER BY entries.entity",
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map(
                params![
                    snapshot.as_bytes().as_slice(),
                    SessionEntityType::PlanRevision.as_str(),
                    SessionEntityType::PlanGate.as_str(),
                    SessionEntityType::PlanEvidence.as_str()
                ],
                raw_plan_revision,
            )
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    };
    records
        .into_iter()
        .map(|(entity, revision, object_type, payload)| {
            let entity = EntityId::from_bytes(
                entity
                    .try_into()
                    .map_err(|_| "semantic entity identity has an invalid width".to_owned())?,
            );
            let revision = RevisionId::from_bytes(
                revision
                    .try_into()
                    .map_err(|_| "semantic revision identity has an invalid width".to_owned())?,
            );
            let object_type = TypeId::new(object_type)
                .map_err(|error| format!("semantic type is invalid: {error}"))?;
            Ok(scriberig_session::PlanSemanticRevision {
                entity,
                object_type,
                revision,
                payload,
            })
        })
        .collect()
}

/// Reads one raw semantic revision row from SQLite.
///
/// # Parameters
/// - `row`: SQLite row from the exact snapshot join.
///
/// # Returns
/// Entity bytes, revision bytes, object type, and immutable payload.
///
/// # Errors
/// Returns a SQLite conversion error when any selected column is malformed.
fn raw_plan_revision(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(Vec<u8>, Vec<u8>, String, Vec<u8>)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

#[cfg(test)]
mod tests {
    use scriberig_core::{EncodingVersion, EntityId, TypeId};
    use scriberig_session::{Plan, SessionStore, schema::SessionEntityType};
    use scriberig_store::{Revision, RootStore, SnapshotChange};

    use crate::SqliteSessionStore;

    /// Verifies compact reads skip malformed historical and detail payloads.
    #[test]
    fn compact_projection_skips_detail_and_history_payloads() {
        let store = SqliteSessionStore::open_in_memory().expect("open store");
        let session = EntityId::from_bytes([71; 16]);
        let branch = EntityId::from_bytes([72; 16]);
        let plan_id = EntityId::from_bytes([73; 16]);
        let plan = Plan::new(
            plan_id,
            session,
            scriberig_session::ContinuationAnchor::BranchHead { branch },
            "compact semantic projection",
        )
        .expect("create plan");
        store
            .save(&scriberig_session::Session {
                id: session,
                branches: vec![
                    scriberig_session::Branch::new(branch, "root").expect("create branch"),
                ],
                goals: Vec::new(),
                tasks: Vec::new(),
                plans: vec![plan],
                execution: scriberig_session::SessionExecutionSelection::default(),
            })
            .expect("save plan");
        let root = super::super::plan_root(session, plan_id).expect("plan root");
        let snapshot = store
            .semantic_store
            .read_root(&root)
            .expect("root snapshot");
        let malformed = [
            (
                EntityId::from_bytes([74; 16]),
                SessionEntityType::PlanRevision,
            ),
            (EntityId::from_bytes([75; 16]), SessionEntityType::PlanGate),
            (
                EntityId::from_bytes([76; 16]),
                SessionEntityType::PlanEvidence,
            ),
        ]
        .into_iter()
        .map(|(entity, object_type)| {
            let revision = Revision::new(
                EncodingVersion::V1,
                TypeId::new(object_type.as_str()).expect("semantic type"),
                b"{".to_vec(),
            )
            .expect("malformed revision wrapper");
            (entity, revision)
        })
        .collect::<Vec<_>>();
        store
            .semantic_store
            .apply_revisions_and_promote(
                root,
                Some(snapshot),
                snapshot,
                &malformed
                    .iter()
                    .map(|(_, revision)| revision.clone())
                    .collect::<Vec<_>>(),
                &malformed
                    .iter()
                    .map(|(entity, revision)| SnapshotChange::Set(*entity, revision.id()))
                    .collect::<Vec<_>>(),
            )
            .expect("promote malformed irrelevant objects");

        let compact = store
            .load_plan_projection(session, plan_id, None)
            .expect("load compact projection")
            .expect("plan remains present");

        assert!(compact.history.is_empty());
        assert!(compact.gates.is_empty());
        assert!(compact.evidence.is_empty());
    }
}
