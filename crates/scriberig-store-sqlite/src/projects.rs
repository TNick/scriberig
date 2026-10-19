//! Persists the multi-project catalog in the shared SQLite database.

use crate::SqliteStore;
use rusqlite::{OptionalExtension, params};
use scriberig_core::{ProjectId, SnapshotId};
use std::path::{Path, PathBuf};

/// Current lifecycle state of an onboarded project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectStatus {
    /// The project has been registered but not imported.
    Registered,
    /// A scan has identified files needing review.
    ScanReady,
    /// The project has a promoted semantic snapshot.
    Imported,
    /// The most recent scan or import reported a failure.
    Failed,
}

impl ProjectStatus {
    /// Converts the status into its durable representation.
    ///
    /// The stable database status value.
    /// # Returns
    /// The operation result.
    fn as_str(&self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::ScanReady => "scan_ready",
            Self::Imported => "imported",
            Self::Failed => "failed",
        }
    }

    /// Parses a durable project status.
    ///
    /// - `value`: Stored status value.
    ///
    /// The recognized status, defaulting to registered for unknown values.
    /// # Returns
    /// The operation result.
    fn parse(value: &str) -> Self {
        match value {
            "scan_ready" => Self::ScanReady,
            "imported" => Self::Imported,
            "failed" => Self::Failed,
            _ => Self::Registered,
        }
    }
}

/// Describes one project stored in a database.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRecord {
    /// Stable project identity.
    pub id: ProjectId,
    /// Human-readable project name.
    pub name: String,
    /// Current canonical filesystem path.
    pub path: PathBuf,
    /// Current onboarding state.
    pub status: ProjectStatus,
    /// Current promoted snapshot, when imported.
    pub current_snapshot: Option<SnapshotId>,
}

impl SqliteStore {
    /// Lists every project in deterministic name/path order.
    ///
    /// Project records stored in this database.
    ///
    /// Returns a SQLite query error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn list_projects(&self) -> rusqlite::Result<Vec<ProjectRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut statement = connection.prepare(
            "SELECT id,name,path,status,current_snapshot FROM projects ORDER BY name,path",
        )?;
        let rows = statement.query_map([], project_from_row)?;
        rows.collect()
    }

    /// Registers a project or returns the existing project at the path.
    ///
    /// - `name`: Human-readable project name.
    /// - `path`: Canonical filesystem path.
    ///
    /// The existing or newly-created project record.
    ///
    /// Returns a SQLite error when the catalog cannot be updated.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn register_project(
        &self,
        name: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> rusqlite::Result<ProjectRecord> {
        let path = normalize_path(path.as_ref());
        if let Some(project) = self.project_by_path(&path)? {
            return Ok(project);
        }
        let project = ProjectRecord {
            id: ProjectId::fresh(),
            name: name.into(),
            path,
            status: ProjectStatus::Registered,
            current_snapshot: None,
        };
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "INSERT INTO projects(id,name,path,created_at,updated_at,status)
             VALUES (?,?,?,datetime('now'),datetime('now'),?)",
            params![
                project.id.as_bytes().as_slice(),
                project.name,
                project.path.to_string_lossy().as_ref(),
                project.status.as_str(),
            ],
        )?;
        Ok(project)
    }

    /// Finds a project by stable identity.
    ///
    /// - `id`: Project identity to find.
    ///
    /// The matching project, when present.
    ///
    /// Returns a SQLite query error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn project(&self, id: ProjectId) -> rusqlite::Result<Option<ProjectRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection
            .query_row(
                "SELECT id,name,path,status,current_snapshot FROM projects WHERE id=?",
                params![id.as_bytes().as_slice()],
                project_from_row,
            )
            .optional()
    }

    /// Updates a project path while preserving its stable identity.
    ///
    /// - `id`: Project identity to update.
    /// - `path`: New canonical filesystem path.
    ///
    /// Unit after the path is updated.
    ///
    /// Returns a SQLite update or uniqueness error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn relink_project(&self, id: ProjectId, path: impl AsRef<Path>) -> rusqlite::Result<()> {
        let path = normalize_path(path.as_ref());
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "UPDATE projects SET path=?,updated_at=datetime('now') WHERE id=?",
            params![path.to_string_lossy().as_ref(), id.as_bytes().as_slice()],
        )?;
        Ok(())
    }

    /// Sets the lifecycle state and current snapshot for a project.
    ///
    /// - `id`: Project identity to update.
    /// - `status`: New lifecycle state.
    /// - `snapshot`: Current promoted snapshot, when available.
    ///
    /// Unit after the update is stored.
    ///
    /// Returns a SQLite update error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn set_project_status(
        &self,
        id: ProjectId,
        status: ProjectStatus,
        snapshot: Option<SnapshotId>,
    ) -> rusqlite::Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "UPDATE projects SET status=?,current_snapshot=?,updated_at=datetime('now') WHERE id=?",
            params![
                status.as_str(),
                snapshot.map(|value| value.as_bytes().to_vec()),
                id.as_bytes().as_slice(),
            ],
        )?;
        Ok(())
    }

    /// Finds a project by normalized path.
    ///
    /// - `path`: Normalized path to find.
    ///
    /// The matching project, when present.
    ///
    /// Returns a SQLite query error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn project_by_path(&self, path: &Path) -> rusqlite::Result<Option<ProjectRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection
            .query_row(
                "SELECT id,name,path,status,current_snapshot FROM projects WHERE path=?",
                params![path.to_string_lossy().as_ref()],
                project_from_row,
            )
            .optional()
    }
}

/// Normalizes a path without requiring it to exist yet.
///
/// - `path`: Path to normalize.
///
/// The canonical path when available, otherwise the supplied path.
/// # Returns
/// The operation result.
fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Decodes one SQLite project row.
///
/// - `row`: SQLite row containing project columns.
///
/// A decoded project record.
///
/// Returns a conversion error for malformed stored identifiers.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
    let id: Vec<u8> = row.get(0)?;
    let path: String = row.get(2)?;
    let status: String = row.get(3)?;
    let snapshot: Option<Vec<u8>> = row.get(4)?;
    let id = id.try_into().map(ProjectId::from_bytes).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Blob,
            Box::new(std::io::Error::other("invalid project id")),
        )
    })?;
    let current_snapshot = snapshot.and_then(|bytes| bytes.try_into().ok().map(SnapshotId::new));
    Ok(ProjectRecord {
        id,
        name: row.get(1)?,
        path: PathBuf::from(path),
        status: ProjectStatus::parse(&status),
        current_snapshot,
    })
}
