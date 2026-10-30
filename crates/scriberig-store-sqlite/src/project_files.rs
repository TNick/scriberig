//! Persists project scan manifests and semantic entity ownership.

use crate::SqliteStore;
use crate::project_rows::{entity_from_row, file_from_row};
use rusqlite::{OptionalExtension, params};
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
    /// Adapter-classified logical grouping key, when one is nominated.
    pub grouping_key: Option<String>,
}

/// Describes exact persisted file evidence without loading its source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectFileDescriptor {
    /// Owning project identity.
    pub project_id: ProjectId,
    /// Stable project-relative POSIX path.
    pub relative_path: String,
    /// Stable source-artifact entity identity.
    pub entity: EntityId,
    /// BLAKE3 digest of the exact source bytes.
    pub digest: [u8; 32],
    /// Selected domain adapter identifier.
    pub adapter: String,
    /// Scan classification such as `ready` or `unsupported`.
    pub status: String,
    /// Adapter-classified logical grouping key, when one is nominated.
    pub grouping_key: Option<String>,
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
    /// - `project`: Project receiving the manifest.
    /// - `files`: Deterministically ordered scan entries.
    ///
    /// Unit after the manifest is durably replaced.
    ///
    /// Returns a SQLite transaction error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
                 (project_id,relative_path,entity,digest,bytes,adapter,status,grouping_key)
                 VALUES (?,?,?,?,?,?,?,?)",
                params![
                    file.project_id.as_bytes().as_slice(),
                    file.relative_path,
                    file.entity.as_bytes().as_slice(),
                    file.digest.as_slice(),
                    file.bytes,
                    file.adapter,
                    file.status,
                    file.grouping_key,
                ],
            )?;
        }
        transaction.commit()
    }

    /// Lists the latest persisted manifest for one project.
    ///
    /// - `project`: Project whose manifest is requested.
    ///
    /// Manifest entries in relative-path order.
    ///
    /// Returns a SQLite query or identifier conversion error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn project_files(&self, project: ProjectId) -> rusqlite::Result<Vec<ProjectFileRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut statement = connection.prepare(
            "SELECT project_id,relative_path,entity,digest,bytes,adapter,status,grouping_key
             FROM project_files WHERE project_id=? ORDER BY relative_path",
        )?;
        let rows = statement.query_map(params![project.as_bytes().as_slice()], file_from_row)?;
        rows.collect()
    }

    /// Lists persisted scan descriptors without loading potentially large bytes.
    ///
    /// - `project`: Project whose manifest descriptors are requested.
    ///
    /// # Returns
    /// Descriptors in deterministic relative-path order.
    ///
    /// # Errors
    /// Returns an error when the query fails or a stored identifier is malformed.
    pub fn project_file_descriptors(
        &self,
        project: ProjectId,
    ) -> rusqlite::Result<Vec<ProjectFileDescriptor>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut statement = connection.prepare(
            "SELECT project_id,relative_path,entity,digest,adapter,status,grouping_key
             FROM project_files WHERE project_id=? ORDER BY relative_path",
        )?;
        let rows = statement.query_map(params![project.as_bytes().as_slice()], |row| {
            let project_bytes: Vec<u8> = row.get(0)?;
            let entity_bytes: Vec<u8> = row.get(2)?;
            let digest_bytes: Vec<u8> = row.get(3)?;
            Ok(ProjectFileDescriptor {
                project_id: ProjectId::from_bytes(project_bytes.try_into().map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(std::io::Error::other("invalid project identifier")),
                    )
                })?),
                relative_path: row.get(1)?,
                entity: EntityId::from_bytes(entity_bytes.try_into().map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Blob,
                        Box::new(std::io::Error::other("invalid entity identifier")),
                    )
                })?),
                digest: digest_bytes.try_into().map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Blob,
                        Box::new(std::io::Error::other("invalid file digest")),
                    )
                })?,
                adapter: row.get(4)?,
                status: row.get(5)?,
                grouping_key: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    /// Lists semantic entities owned by one project.
    ///
    /// - `project`: Project whose entities are requested.
    ///
    /// Entity ownership records in deterministic order.
    ///
    /// Returns a SQLite query or identifier conversion error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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

    /// Finds a bounded set of project entities matching semantic text.
    ///
    /// - `project`: Project whose entities are searched.
    /// - `query`: Case-insensitive path, kind, or name substring.
    /// - `limit`: Maximum number of matching entities returned.
    ///
    /// Matching ownership records in deterministic path and identity order.
    ///
    /// Returns a SQLite query or identifier conversion error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn find_project_entities(
        &self,
        project: ProjectId,
        query: &str,
        limit: usize,
    ) -> rusqlite::Result<Vec<ProjectEntityRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut statement = connection.prepare(
            "SELECT project_id,relative_path,entity,kind,name
             FROM project_entities
             WHERE project_id=? AND (lower(relative_path) LIKE ? OR lower(kind) LIKE ? OR lower(name) LIKE ?)
             ORDER BY CASE WHEN kind='source' THEN 0 ELSE 1 END,
                      CASE WHEN name IS NOT NULL THEN 0 ELSE 1 END,
                      relative_path,entity LIMIT ?",
        )?;
        let pattern = format!("%{}%", query.to_lowercase());
        let rows = statement.query_map(
            params![
                project.as_bytes().as_slice(),
                pattern,
                pattern,
                pattern,
                i64::try_from(limit).unwrap_or(i64::MAX),
            ],
            entity_from_row,
        )?;
        rows.collect()
    }

    /// Loads one semantic entity ownership record by project and identity.
    ///
    /// - `project`: Project that must own the entity.
    /// - `entity`: Stable semantic entity identity.
    ///
    /// The ownership record when it belongs to the project.
    ///
    /// Returns a SQLite query or identifier conversion error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn project_entity(
        &self,
        project: ProjectId,
        entity: EntityId,
    ) -> rusqlite::Result<Option<ProjectEntityRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection
            .query_row(
                "SELECT project_id,relative_path,entity,kind,name
                 FROM project_entities WHERE project_id=? AND entity=?",
                params![project.as_bytes().as_slice(), entity.as_bytes().as_slice()],
                entity_from_row,
            )
            .optional()
    }

    /// Finds the preferred semantic root source owned by a project.
    ///
    /// - `project`: Project whose root source is requested.
    ///
    /// The `src/lib.rs` source when present, otherwise the first source path.
    ///
    /// Returns a SQLite query or identifier conversion error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn project_source_root(
        &self,
        project: ProjectId,
    ) -> rusqlite::Result<Option<ProjectEntityRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection
            .query_row(
                "SELECT project_id,relative_path,entity,kind,name FROM project_entities
                 WHERE project_id=? AND kind='source'
                 ORDER BY CASE WHEN relative_path='src/lib.rs' THEN 0 ELSE 1 END,
                          relative_path,entity LIMIT 1",
                params![project.as_bytes().as_slice()],
                entity_from_row,
            )
            .optional()
    }

    /// Replaces semantic entity ownership after a successful import.
    ///
    /// - `project`: Project receiving ownership records.
    /// - `entities`: Current entities grouped by source path.
    ///
    /// Unit after ownership is durably replaced.
    ///
    /// Returns a SQLite transaction error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
