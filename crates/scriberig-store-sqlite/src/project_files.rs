//! Persists project scan manifests and semantic entity ownership.

use crate::SqliteStore;
use crate::project_rows::{entity_from_row, file_from_row};
use rusqlite::params;
use scriberig_core::{EntityId, ProjectId};

/// Describes one file captured by a project scan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectFileRecord {
    /// Owning project identity.
    pub project_id: ProjectId,
    /// Stable project-relative POSIX path.
    pub relative_path: String,
    /// Stable source-artifact entity identity.
    pub entity: EntityId,
    /// BLAKE3 digest of the exact source bytes.
    pub digest: [u8; 32],
    /// Exact source bytes captured during the scan.
    pub bytes: Vec<u8>,
    /// Selected domain adapter identifier.
    pub adapter: String,
    /// Scan classification such as `ready` or `unsupported`.
    pub status: String,
}

/// Describes one semantic entity currently owned by a project source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectEntityRecord {
    /// Owning project identity.
    pub project_id: ProjectId,
    /// Project-relative source path.
    pub relative_path: String,
    /// Semantic entity identity.
    pub entity: EntityId,
    /// Stable entity-kind label.
    pub kind: String,
    /// Human-readable semantic name when the entity is a named symbol.
    pub name: Option<String>,
}

impl SqliteStore {
    /// Replaces the persisted manifest for one project scan.
    ///
    /// # Parameters
    /// - `project`: Project receiving the manifest.
    /// - `files`: Deterministically ordered scan entries.
    ///
    /// # Returns
    /// Unit after the manifest is durably replaced.
    ///
    /// # Errors
    /// Returns a SQLite transaction error.
    pub fn replace_project_files(
        &self,
        project: ProjectId,
        files: &[ProjectFileRecord],
    ) -> rusqlite::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM project_files WHERE project_id=?",
            params![project.as_bytes().as_slice()],
        )?;
        for file in files {
            transaction.execute(
                "INSERT INTO project_files
                 (project_id,relative_path,entity,digest,bytes,adapter,status)
                 VALUES (?,?,?,?,?,?,?)",
                params![
                    file.project_id.as_bytes().as_slice(),
                    file.relative_path,
                    file.entity.as_bytes().as_slice(),
                    file.digest.as_slice(),
                    file.bytes,
                    file.adapter,
                    file.status,
                ],
            )?;
        }
        transaction.commit()
    }

    /// Lists the latest persisted manifest for one project.
    ///
    /// # Parameters
    /// - `project`: Project whose manifest is requested.
    ///
    /// # Returns
    /// Manifest entries in relative-path order.
    ///
    /// # Errors
    /// Returns a SQLite query or identifier conversion error.
    pub fn project_files(&self, project: ProjectId) -> rusqlite::Result<Vec<ProjectFileRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut statement = connection.prepare(
            "SELECT project_id,relative_path,entity,digest,bytes,adapter,status
             FROM project_files WHERE project_id=? ORDER BY relative_path",
        )?;
        let rows = statement.query_map(params![project.as_bytes().as_slice()], file_from_row)?;
        rows.collect()
    }

    /// Lists semantic entities owned by one project.
    ///
    /// # Parameters
    /// - `project`: Project whose entities are requested.
    ///
    /// # Returns
    /// Entity ownership records in deterministic order.
    ///
    /// # Errors
    /// Returns a SQLite query or identifier conversion error.
    pub fn project_entities(
        &self,
        project: ProjectId,
    ) -> rusqlite::Result<Vec<ProjectEntityRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut statement = connection.prepare(
            "SELECT project_id,relative_path,entity,kind,name
             FROM project_entities WHERE project_id=? ORDER BY relative_path,entity",
        )?;
        let rows = statement.query_map(params![project.as_bytes().as_slice()], entity_from_row)?;
        rows.collect()
    }

    /// Replaces semantic entity ownership after a successful import.
    ///
    /// # Parameters
    /// - `project`: Project receiving ownership records.
    /// - `entities`: Current entities grouped by source path.
    ///
    /// # Returns
    /// Unit after ownership is durably replaced.
    ///
    /// # Errors
    /// Returns a SQLite transaction error.
    pub fn replace_project_entities(
        &self,
        project: ProjectId,
        entities: &[ProjectEntityRecord],
    ) -> rusqlite::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM project_entities WHERE project_id=?",
            params![project.as_bytes().as_slice()],
        )?;
        for entity in entities {
            transaction.execute(
                "INSERT INTO project_entities(project_id,relative_path,entity,kind,name)
                 VALUES (?,?,?,?,?)",
                params![
                    entity.project_id.as_bytes().as_slice(),
                    entity.relative_path,
                    entity.entity.as_bytes().as_slice(),
                    entity.kind,
                    entity.name,
                ],
            )?;
        }
        transaction.commit()
    }
}
