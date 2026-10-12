//! Persists project candidates and filesystem effect receipts.

use crate::SqliteStore;
use rusqlite::{OptionalExtension, params};
use scriberig_core::{EntityId, ProjectId, SnapshotId};

/// Durable source candidate data independent of the project service type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCandidateRecord {
    /// Stable candidate identity.
    pub id: String,
    /// Candidate project.
    pub project: ProjectId,
    /// Exact candidate base snapshot.
    pub base_snapshot: SnapshotId,
    /// Selected semantic entity.
    pub entity: EntityId,
    /// Candidate-relative source path.
    pub path: String,
    /// Source bytes at candidate creation.
    pub old: Vec<u8>,
    /// Proposed source bytes.
    pub new: Vec<u8>,
    /// Validation state (`passed` or `failed`).
    pub validation: String,
    /// Newline-separated validation diagnostics.
    pub diagnostics: String,
}

/// Durable state for a candidate's filesystem effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectEffectRecord {
    /// Candidate whose effect is being tracked.
    pub candidate: String,
    /// Effect project.
    pub project: ProjectId,
    /// Candidate base snapshot.
    pub base_snapshot: SnapshotId,
    /// Promoted snapshot, when canonical application completed.
    pub successor_snapshot: Option<SnapshotId>,
    /// Materialized project-relative path.
    pub path: String,
    /// Effect state such as `attempted`, `applied`, or `failed`.
    pub state: String,
    /// Safe diagnostic for failed or unknown effects.
    pub diagnostic: Option<String>,
}

impl SqliteStore {
    /// Stores or replaces a durable project candidate.
    ///
    /// # Parameters
    /// - `candidate`: Candidate data to persist.
    ///
    /// # Returns
    /// Unit after the candidate is durable.
    ///
    /// # Errors
    /// Returns a SQLite transaction error.
    pub fn put_project_candidate(
        &self,
        candidate: &ProjectCandidateRecord,
    ) -> rusqlite::Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "INSERT INTO project_candidates
             (id,project_id,base_snapshot,entity,relative_path,old_bytes,new_bytes,
              validation,diagnostics,created_at)
             VALUES (?,?,?,?,?,?,?,?,?,datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                 validation=excluded.validation,
                 diagnostics=excluded.diagnostics",
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
            ],
        )?;
        Ok(())
    }

    /// Loads one durable project candidate by identity.
    ///
    /// # Parameters
    /// - `id`: Candidate identity.
    ///
    /// # Returns
    /// The candidate when it exists.
    ///
    /// # Errors
    /// Returns a SQLite query or identifier conversion error.
    pub fn project_candidate(&self, id: &str) -> rusqlite::Result<Option<ProjectCandidateRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection
            .query_row(
                "SELECT id,project_id,base_snapshot,entity,relative_path,old_bytes,new_bytes,
                        validation,diagnostics
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
                    })
                },
            )
            .optional()
    }

    /// Records the current external materialization effect state.
    ///
    /// # Parameters
    /// - `effect`: Effect receipt to persist.
    ///
    /// # Returns
    /// Unit after the receipt is durable.
    ///
    /// # Errors
    /// Returns a SQLite transaction error.
    pub fn put_project_effect(&self, effect: &ProjectEffectRecord) -> rusqlite::Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "INSERT INTO project_effects
             (candidate_id,project_id,base_snapshot,successor_snapshot,relative_path,
              state,diagnostic,updated_at)
             VALUES (?,?,?,?,?,?,?,datetime('now'))
             ON CONFLICT(candidate_id) DO UPDATE SET
                 successor_snapshot=excluded.successor_snapshot,
                 state=excluded.state,
                 diagnostic=excluded.diagnostic,
                 updated_at=excluded.updated_at",
            params![
                effect.candidate,
                effect.project.as_bytes().as_slice(),
                effect.base_snapshot.as_bytes().as_slice(),
                effect
                    .successor_snapshot
                    .map(|snapshot| snapshot.as_bytes().to_vec()),
                effect.path,
                effect.state,
                effect.diagnostic,
            ],
        )?;
        Ok(())
    }

    /// Loads the effect receipt for one candidate.
    ///
    /// # Parameters
    /// - `candidate`: Candidate identity.
    ///
    /// # Returns
    /// The durable effect receipt when present.
    ///
    /// # Errors
    /// Returns a SQLite query or identifier conversion error.
    pub fn project_effect(&self, candidate: &str) -> rusqlite::Result<Option<ProjectEffectRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection
            .query_row(
                "SELECT candidate_id,project_id,base_snapshot,successor_snapshot,
                        relative_path,state,diagnostic
                 FROM project_effects WHERE candidate_id=?",
                [candidate],
                |row| {
                    let successor: Option<Vec<u8>> = row.get(3)?;
                    Ok(ProjectEffectRecord {
                        candidate: row.get(0)?,
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
                        successor_snapshot: successor
                            .map(|bytes| bytes.try_into().map(SnapshotId::new))
                            .transpose()
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        path: row.get(4)?,
                        state: row.get(5)?,
                        diagnostic: row.get(6)?,
                    })
                },
            )
            .optional()
    }
}
