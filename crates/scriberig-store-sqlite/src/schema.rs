//! Defines and initializes the SQLite storage schema.

use std::error::Error;
use std::fmt::{Display, Formatter};

use rusqlite::{Connection, Transaction};

/// Current SQLite backend schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 4;

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
    /// # Parameters
    /// - `self`: Schema version error.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
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
    /// # Parameters
    /// - `self`: Metadata validation error.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
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
/// # Parameters
/// - `connection`: SQLite connection to initialize.
///
/// # Returns
/// The detected backend schema version.
///
/// # Errors
/// Returns an SQLite error or an explicit unsupported-version error.
pub fn initialize(connection: &mut Connection) -> rusqlite::Result<u32> {
    let transaction = connection.transaction()?;
    initialize_schema(&transaction)?;
    transaction.execute(
        "UPDATE metadata SET value=? WHERE key='schema_version' AND value IN ('1','2','3')",
        [CURRENT_SCHEMA_VERSION.to_string()],
    )?;
    let has_name = transaction
        .prepare("PRAGMA table_info(project_entities)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .any(|column| column == "name");
    if !has_name {
        transaction.execute("ALTER TABLE project_entities ADD COLUMN name TEXT", [])?;
    }
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
/// # Parameters
/// - `transaction`: Transaction receiving schema creation.
///
/// # Returns
/// Unit after schema creation.
///
/// # Errors
/// Returns the SQLite schema error.
#[allow(clippy::too_many_lines)]
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
             VALUES ('schema_version', '4');
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
/// # Parameters
/// - `transaction`: Transaction containing metadata.
/// - `key`: Metadata key to read.
///
/// # Returns
/// The parsed metadata version.
///
/// # Errors
/// Returns a SQLite query or parse error.
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
/// # Parameters
/// - `transaction`: Transaction containing metadata.
/// - `key`: Metadata key to validate.
/// - `expected`: Required value.
///
/// # Returns
/// Unit when the value matches.
///
/// # Errors
/// Returns a SQLite query or metadata compatibility error.
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
