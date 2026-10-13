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
    /// Most recent registered validation profile, when the candidate was checked.
    pub validation_profile: Option<String>,
    /// Ordered project source effects belonging to this candidate.
    pub effects: Vec<ProjectSourceEffectRecord>,
}

/// One durable source effect in an aggregate project candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSourceEffectRecord {
    /// Project-relative path owned by the candidate.
    pub path: String,
    /// Effect kind: `create`, `update`, or `delete`.
    pub kind: String,
    /// Exact base bytes, absent for a create.
    pub old: Option<Vec<u8>>,
    /// Proposed bytes, absent for a delete.
    pub new: Option<Vec<u8>>,
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

/// Durable receipt for one project-relative candidate effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectEffectPathRecord {
    /// Candidate whose effect is tracked.
    pub candidate: String,
    /// Project-relative affected path.
    pub path: String,
    /// State such as `not_attempted`, `applied`, or `conflict`.
    pub state: String,
    /// BLAKE3 digest expected from the exact base, when present.
    pub expected_digest: Option<[u8; 32]>,
    /// BLAKE3 digest observed after the effect, when present.
    pub observed_digest: Option<[u8; 32]>,
    /// Bounded safe diagnostic.
    pub diagnostic: Option<String>,
}

/// Converts an optional SQLite blob into an exact BLAKE3 digest.
///
/// # Parameters
/// - `bytes`: Stored digest bytes.
///
/// # Returns
/// A digest when the column is populated.
///
/// # Errors
/// Returns a query conversion error for an invalid digest length.
fn digest_from_column(bytes: Option<Vec<u8>>) -> rusqlite::Result<Option<[u8; 32]>> {
    bytes
        .map(|value| value.try_into().map_err(|_| rusqlite::Error::InvalidQuery))
        .transpose()
}

impl SqliteStore {
    /// Persists the latest durable state for one candidate path effect.
    ///
    /// # Parameters
    /// - `effect`: Per-path receipt to persist.
    ///
    /// # Returns
    /// Unit after the receipt is durable.
    ///
    /// # Errors
    /// Returns a SQLite transaction or digest conversion error.
    pub fn put_project_effect_path(
        &self,
        effect: &ProjectEffectPathRecord,
    ) -> rusqlite::Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "INSERT INTO project_effect_paths
             (candidate_id,relative_path,state,expected_digest,observed_digest,diagnostic,updated_at)
             VALUES (?,?,?,?,?,?,datetime('now'))
             ON CONFLICT(candidate_id,relative_path) DO UPDATE SET
                 state=excluded.state,
                 expected_digest=excluded.expected_digest,
                 observed_digest=excluded.observed_digest,
                 diagnostic=excluded.diagnostic,
                 updated_at=excluded.updated_at",
            params![
                effect.candidate,
                effect.path,
                effect.state,
                effect.expected_digest.map(|value| value.to_vec()),
                effect.observed_digest.map(|value| value.to_vec()),
                effect.diagnostic,
            ],
        )?;
        Ok(())
    }

    /// Loads path-level receipts for one aggregate candidate.
    ///
    /// # Parameters
    /// - `candidate`: Stable candidate identity.
    ///
    /// # Returns
    /// Deterministically ordered effect receipts.
    ///
    /// # Errors
    /// Returns a SQLite query or digest conversion error.
    pub fn project_effect_paths(
        &self,
        candidate: &str,
    ) -> rusqlite::Result<Vec<ProjectEffectPathRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut statement = connection.prepare(
            "SELECT candidate_id,relative_path,state,expected_digest,observed_digest,diagnostic
             FROM project_effect_paths WHERE candidate_id=? ORDER BY relative_path",
        )?;
        statement
            .query_map([candidate], |row| {
                Ok(ProjectEffectPathRecord {
                    candidate: row.get(0)?,
                    path: row.get(1)?,
                    state: row.get(2)?,
                    expected_digest: digest_from_column(row.get(3)?)?,
                    observed_digest: digest_from_column(row.get(4)?)?,
                    diagnostic: row.get(5)?,
                })
            })?
            .collect()
    }

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
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO project_candidates
             (id,project_id,base_snapshot,entity,relative_path,old_bytes,new_bytes,
              validation,diagnostics,validation_profile,created_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                 validation=excluded.validation,
                 diagnostics=excluded.diagnostics,
                 validation_profile=excluded.validation_profile",
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
                 (candidate_id,ordinal,relative_path,kind,old_bytes,new_bytes)
                 VALUES (?,?,?,?,?,?)",
                params![
                    candidate.id,
                    ordinal,
                    effect.path,
                    effect.kind,
                    effect.old,
                    effect.new
                ],
            )?;
        }
        transaction.commit()?;
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
        let mut candidate = connection
            .query_row(
                "SELECT id,project_id,base_snapshot,entity,relative_path,old_bytes,new_bytes,
                        validation,diagnostics,validation_profile
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
                        effects: Vec::new(),
                    })
                },
            )
            .optional()?;
        if let Some(candidate) = &mut candidate {
            let mut statement = connection.prepare(
                "SELECT relative_path,kind,old_bytes,new_bytes
                 FROM project_candidate_effects WHERE candidate_id=? ORDER BY ordinal",
            )?;
            candidate.effects = statement
                .query_map([id], |row| {
                    Ok(ProjectSourceEffectRecord {
                        path: row.get(0)?,
                        kind: row.get(1)?,
                        old: row.get(2)?,
                        new: row.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
        }
        Ok(candidate)
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
