//! Stores and loads durable project candidate records.

use crate::SqliteStore;
use rusqlite::{OptionalExtension, params};
use scriberig_core::{EntityId, ProjectId, SnapshotId};

use super::workflow::{ProjectCandidateRecord, ProjectSourceEffectRecord};

impl SqliteStore {
    /// Stores or replaces a durable project candidate.
    ///
    /// - `candidate`: Candidate data to persist.
    ///
    /// Unit after the candidate is durable.
    ///
    /// Returns a SQLite transaction error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn put_project_candidate(
        &self,
        candidate: &ProjectCandidateRecord,
    ) -> rusqlite::Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO project_candidates
             (id,project_id,base_snapshot,entity,relative_path,old_bytes,new_bytes,
              validation,diagnostics,validation_profile,validation_digest,semantic_operation,semantic_target,semantic_changes,created_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                 validation=excluded.validation,
                 diagnostics=excluded.diagnostics,
                 validation_profile=excluded.validation_profile,
                 validation_digest=excluded.validation_digest,
                 semantic_operation=excluded.semantic_operation,
                 semantic_target=excluded.semantic_target,
                 semantic_changes=excluded.semantic_changes",
            params![
                candidate.id,
                candidate.project.as_bytes().as_slice(),
                candidate.base_snapshot.as_bytes().as_slice(),
                candidate.entity.as_bytes().as_slice(),
                candidate.path,
                candidate.old,
                candidate.new,
                candidate.validation,
                candidate.diagnostics,
                candidate.validation_profile,
                candidate.validation_digest,
                candidate.semantic_operation,
                candidate.semantic_target.map(|entity| entity.as_bytes().to_vec()),
                candidate.semantic_changes,
            ],
        )?;
        transaction.execute(
            "DELETE FROM project_candidate_effects WHERE candidate_id=?",
            [&candidate.id],
        )?;
        for (ordinal, effect) in candidate.effects.iter().enumerate() {
            let ordinal = i64::try_from(ordinal).map_err(|_| rusqlite::Error::InvalidQuery)?;
            transaction.execute(
                "INSERT INTO project_candidate_effects
                 (candidate_id,ordinal,relative_path,kind,old_bytes,new_bytes,observed_bytes)
                 VALUES (?,?,?,?,?,?,?)",
                params![
                    candidate.id,
                    ordinal,
                    effect.path,
                    effect.kind,
                    effect.old,
                    effect.new,
                    effect.observed
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Loads one durable project candidate by identity.
    ///
    /// - `id`: Candidate identity.
    ///
    /// The candidate when it exists.
    ///
    /// Returns a SQLite query or identifier conversion error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn project_candidate(&self, id: &str) -> rusqlite::Result<Option<ProjectCandidateRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut candidate = connection
            .query_row(
                "SELECT id,project_id,base_snapshot,entity,relative_path,old_bytes,new_bytes,
                        validation,diagnostics,validation_profile,validation_digest,semantic_operation,semantic_target,semantic_changes
                 FROM project_candidates WHERE id=?",
                [id],
                |row| {
                    Ok(ProjectCandidateRecord {
                        id: row.get(0)?,
                        project: ProjectId::from_bytes(
                            row.get::<_, Vec<u8>>(1)?
                                .try_into()
                                .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        ),
                        base_snapshot: SnapshotId::new(
                            row.get::<_, Vec<u8>>(2)?
                                .try_into()
                                .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        ),
                        entity: EntityId::from_bytes(
                            row.get::<_, Vec<u8>>(3)?
                                .try_into()
                                .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        ),
                        path: row.get(4)?,
                        old: row.get(5)?,
                        new: row.get(6)?,
                        validation: row.get(7)?,
                        diagnostics: row.get(8)?,
                        validation_profile: row.get(9)?,
                        validation_digest: row.get(10)?,
                        semantic_operation: row.get(11)?,
                        semantic_target: row
                            .get::<_, Option<Vec<u8>>>(12)?
                            .map(|bytes| {
                                bytes
                                    .try_into()
                                    .map(EntityId::from_bytes)
                                    .map_err(|_| rusqlite::Error::InvalidQuery)
                            })
                            .transpose()?,
                        semantic_changes: row.get(13)?,
                        effects: Vec::new(),
                    })
                },
            )
            .optional()?;
        if let Some(candidate) = &mut candidate {
            let mut statement = connection.prepare(
                "SELECT relative_path,kind,old_bytes,new_bytes,observed_bytes
                 FROM project_candidate_effects WHERE candidate_id=? ORDER BY ordinal",
            )?;
            candidate.effects = statement
                .query_map([id], |row| {
                    Ok(ProjectSourceEffectRecord {
                        path: row.get(0)?,
                        kind: row.get(1)?,
                        old: row.get(2)?,
                        new: row.get(3)?,
                        observed: row.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
        }
        Ok(candidate)
    }
}
