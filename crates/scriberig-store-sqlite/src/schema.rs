//! Defines and initializes the SQLite storage schema.

use std::error::Error;
use std::fmt::{Display, Formatter};

use rusqlite::{Connection, Transaction};

#[path = "schema_migrations.rs"]
mod migrations;
#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;

/// Current SQLite backend schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 11;

const INTEGRITY_FORMAT_VERSION: &str = "1";
const RECOVERY_MODE: &str = "transactional-root-promotion";

/// Reports a backend schema that this build cannot safely open.
#[derive(Debug, Eq, PartialEq)]
pub struct UnsupportedSchemaVersion {
    /// Version found in the database.
    pub found: u32,
}

impl Display for UnsupportedSchemaVersion {
    /// Formats the schema version failure.
    ///
    /// - `self`: Schema version error.
    /// - `formatter`: Formatting destination.
    ///
    /// A formatting result.
    ///
    /// Returns a formatting error when the destination rejects output.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "unsupported SQLite backend schema version {}",
            self.found
        )
    }
}

impl Error for UnsupportedSchemaVersion {}

/// Reports incompatible SQLite integrity or recovery metadata.
#[derive(Debug, Eq, PartialEq)]
pub struct UnsupportedMetadata {
    /// Metadata key that failed validation.
    pub key: String,
    /// Value found in the database.
    pub found: String,
}

impl Display for UnsupportedMetadata {
    /// Formats the metadata validation failure.
    ///
    /// - `self`: Metadata validation error.
    /// - `formatter`: Formatting destination.
    ///
    /// A formatting result.
    ///
    /// Returns a formatting error when the destination rejects output.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "unsupported SQLite metadata {}={}",
            self.key, self.found
        )
    }
}

impl Error for UnsupportedMetadata {}

/// Initializes the schema and verifies its backend version.
///
/// - `connection`: SQLite connection to initialize.
///
/// The detected backend schema version.
///
/// Returns an SQLite error or an explicit unsupported-version error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn initialize(connection: &mut Connection) -> rusqlite::Result<u32> {
    let transaction = connection.transaction()?;
    initialize_schema(&transaction)?;
    transaction.execute(
        "UPDATE metadata SET value=? WHERE key='schema_version' AND value IN ('1','2','3','4','5','6','7','8','9','10')",
        [CURRENT_SCHEMA_VERSION.to_string()],
    )?;
    migrations::upgrade_optional_columns(&transaction)?;
    let version = metadata_u32(&transaction, "schema_version")?;
    if version > CURRENT_SCHEMA_VERSION || version == 0 {
        return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
            UnsupportedSchemaVersion { found: version },
        )));
    }
    validate_metadata(
        &transaction,
        "integrity_format_version",
        INTEGRITY_FORMAT_VERSION,
    )?;
    validate_metadata(&transaction, "recovery_mode", RECOVERY_MODE)?;
    transaction.commit()?;
    Ok(version)
}

/// Creates the version-one backend tables and metadata within a transaction.
///
/// - `transaction`: Transaction receiving schema creation.
///
/// Unit after schema creation.
///
/// Returns the SQLite schema error.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn initialize_schema(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS metadata (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS revisions (
             id BLOB PRIMARY KEY,
             version INTEGER NOT NULL,
             object_type TEXT NOT NULL,
             payload BLOB NOT NULL
         );
         CREATE TABLE IF NOT EXISTS snapshots (id BLOB PRIMARY KEY);
         CREATE TABLE IF NOT EXISTS snapshot_parents (
             snapshot_id BLOB NOT NULL,
             parent_id BLOB NOT NULL,
             PRIMARY KEY(snapshot_id, parent_id)
         );
         CREATE TABLE IF NOT EXISTS snapshot_entries (
             snapshot_id BLOB NOT NULL,
             entity BLOB NOT NULL,
             revision BLOB NOT NULL,
             PRIMARY KEY(snapshot_id, entity)
         );
         CREATE TABLE IF NOT EXISTS snapshot_relations (
             snapshot_id BLOB NOT NULL,
             source_entity BLOB NOT NULL,
             relation_type TEXT NOT NULL,
             target_entity BLOB NOT NULL,
             PRIMARY KEY(snapshot_id, source_entity, relation_type, target_entity)
         );
         CREATE TABLE IF NOT EXISTS snapshot_nodes (
             id BLOB PRIMARY KEY,
             kind INTEGER NOT NULL,
             entity BLOB,
             revision BLOB,
             left_child BLOB,
             right_child BLOB,
             item_count INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS snapshot_node_roots (
             snapshot_id BLOB PRIMARY KEY,
             node_id BLOB NOT NULL
         );
         CREATE TABLE IF NOT EXISTS roots (
             name TEXT PRIMARY KEY,
             snapshot_id BLOB NOT NULL
         );
         INSERT OR IGNORE INTO metadata(key, value)
             VALUES ('schema_version', '7');
         INSERT OR IGNORE INTO metadata(key, value)
             VALUES ('integrity_format_version', '1');
         INSERT OR IGNORE INTO metadata(key, value)
             VALUES ('recovery_mode', 'transactional-root-promotion');
         CREATE TABLE IF NOT EXISTS projects (
             id BLOB PRIMARY KEY,
             name TEXT NOT NULL,
             path TEXT NOT NULL UNIQUE,
             created_at TEXT NOT NULL,
             updated_at TEXT NOT NULL,
             current_snapshot BLOB,
             status TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS projects_path_idx ON projects(path);
         CREATE TABLE IF NOT EXISTS project_files (
             project_id BLOB NOT NULL,
             relative_path TEXT NOT NULL,
             entity BLOB NOT NULL,
             digest BLOB NOT NULL,
             bytes BLOB NOT NULL,
             adapter TEXT NOT NULL,
             status TEXT NOT NULL,
             grouping_key TEXT,
             PRIMARY KEY(project_id, relative_path),
             FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS project_files_project_idx
             ON project_files(project_id);
         CREATE TABLE IF NOT EXISTS project_entities (
             project_id BLOB NOT NULL,
             relative_path TEXT NOT NULL,
             entity BLOB NOT NULL,
             kind TEXT NOT NULL,
             name TEXT,
             PRIMARY KEY(project_id, relative_path, entity),
             FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS project_entities_project_idx
             ON project_entities(project_id);
         CREATE TABLE IF NOT EXISTS project_candidates (
             id TEXT PRIMARY KEY,
             project_id BLOB NOT NULL,
             base_snapshot BLOB NOT NULL,
             entity BLOB NOT NULL,
             relative_path TEXT NOT NULL,
             old_bytes BLOB NOT NULL,
             new_bytes BLOB NOT NULL,
             validation TEXT NOT NULL,
             diagnostics TEXT NOT NULL,
             validation_profile TEXT,
             validation_digest BLOB,
             semantic_operation TEXT,
             semantic_target BLOB,
             semantic_changes TEXT,
             created_at TEXT NOT NULL,
             FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS project_candidates_project_idx
             ON project_candidates(project_id);
         CREATE TABLE IF NOT EXISTS project_candidate_effects (
             candidate_id TEXT NOT NULL,
             ordinal INTEGER NOT NULL,
             relative_path TEXT NOT NULL,
             kind TEXT NOT NULL,
             old_bytes BLOB,
             new_bytes BLOB,
             observed_bytes BLOB,
             PRIMARY KEY(candidate_id, ordinal),
             UNIQUE(candidate_id, relative_path),
             FOREIGN KEY(candidate_id) REFERENCES project_candidates(id) ON DELETE CASCADE
         );
         CREATE TABLE IF NOT EXISTS project_effects (
             candidate_id TEXT PRIMARY KEY,
             project_id BLOB NOT NULL,
             base_snapshot BLOB NOT NULL,
             successor_snapshot BLOB,
             relative_path TEXT NOT NULL,
             state TEXT NOT NULL,
             diagnostic TEXT,
             updated_at TEXT NOT NULL,
             FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
         );
         CREATE TABLE IF NOT EXISTS project_effect_paths (
             candidate_id TEXT NOT NULL,
             relative_path TEXT NOT NULL,
             state TEXT NOT NULL,
             expected_digest BLOB,
             observed_digest BLOB,
             diagnostic TEXT,
             updated_at TEXT NOT NULL,
             PRIMARY KEY(candidate_id,relative_path)
         );",
    )
}

/// Reads a numeric metadata value.
///
/// - `transaction`: Transaction containing metadata.
/// - `key`: Metadata key to read.
///
/// The parsed metadata version.
///
/// Returns a SQLite query or parse error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn metadata_u32(transaction: &Transaction<'_>, key: &str) -> rusqlite::Result<u32> {
    transaction.query_row("SELECT value FROM metadata WHERE key=?", [key], |row| {
        let value: String = row.get(0)?;
        value
            .parse::<u32>()
            .map_err(|_| rusqlite::Error::InvalidQuery)
    })
}

/// Validates a required textual metadata value.
///
/// - `transaction`: Transaction containing metadata.
/// - `key`: Metadata key to validate.
/// - `expected`: Required value.
///
/// Unit when the value matches.
///
/// Returns a SQLite query or metadata compatibility error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn validate_metadata(
    transaction: &Transaction<'_>,
    key: &str,
    expected: &str,
) -> rusqlite::Result<()> {
    let found: String =
        transaction.query_row("SELECT value FROM metadata WHERE key=?", [key], |row| {
            row.get(0)
        })?;
    if found != expected {
        return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
            UnsupportedMetadata {
                key: key.to_owned(),
                found,
            },
        )));
    }
    Ok(())
}
