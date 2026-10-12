//! Provides the shared project scan and semantic onboarding service.

use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::Arc;

use scriberig_core::{ProjectId, SnapshotId};
use scriberig_store::{RootStore, Snapshot, SnapshotChange};
use scriberig_store_sqlite::{
    ProjectEntityRecord, ProjectFileRecord, ProjectRecord, ProjectStatus, SqliteStore,
};

#[path = "support.rs"]
mod support;
use support::{import_file, scan_directory};

/// Shared project onboarding service used by CLI, HTTP, and TUI surfaces.
#[derive(Clone, Debug)]
pub struct ProjectService {
    /// Shared durable SQLite catalog and object store.
    store: Arc<SqliteStore>,
}

/// A scanned project file with exact source bytes and classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanFile {
    /// Project-relative POSIX path.
    pub relative_path: String,
    /// Exact source bytes.
    pub bytes: Vec<u8>,
    /// Selected adapter identifier.
    pub adapter: String,
    /// Stable scan classification.
    pub status: String,
}

/// Result of registering and persisting a project scan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanReport {
    /// Registered project record.
    pub project: ProjectRecord,
    /// Persisted deterministic scan files.
    pub files: Vec<ScanFile>,
}

/// Result of promoting one project import.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportReport {
    /// Project after successful import.
    pub project: ProjectRecord,
    /// Number of source files imported.
    pub files: usize,
    /// Number of semantic entities promoted.
    pub entities: usize,
    /// Promoted immutable snapshot.
    pub snapshot: SnapshotId,
}

/// Reports a project onboarding failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectError {
    /// Stable failure category.
    pub code: &'static str,
    /// Safe human-readable failure detail.
    pub message: String,
}

impl Display for ProjectError {
    /// Formats the onboarding failure.
    ///
    /// # Parameters
    /// - `self`: Onboarding failure to format.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProjectError {}

impl ProjectService {
    /// Creates a project service over a shared SQLite store.
    ///
    /// # Parameters
    /// - `store`: Shared durable project and object store.
    ///
    /// # Returns
    /// A project onboarding service.
    #[must_use]
    pub fn new(store: Arc<SqliteStore>) -> Self {
        Self { store }
    }

    /// Registers and persists a deterministic filesystem scan.
    ///
    /// # Parameters
    /// - `name`: Optional display name for a new project.
    /// - `path`: Filesystem project path.
    ///
    /// # Returns
    /// The project record and persisted scan manifest.
    ///
    /// # Errors
    /// Returns a structured filesystem or storage failure.
    pub fn scan(
        &self,
        name: Option<String>,
        path: impl AsRef<Path>,
    ) -> Result<ScanReport, ProjectError> {
        let path = path.as_ref();
        let display_name = name
            .or_else(|| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "project".to_owned());
        let project = self
            .store
            .register_project(display_name, path)
            .map_err(storage_error)?;
        let files = scan_directory(&project.path).map_err(|error| filesystem_error(&error))?;
        let records = files
            .iter()
            .map(|file| ProjectFileRecord {
                project_id: project.id,
                relative_path: file.relative_path.clone(),
                entity: support::source_entity(project.id, &file.relative_path),
                digest: *blake3::hash(&file.bytes).as_bytes(),
                bytes: file.bytes.clone(),
                adapter: file.adapter.clone(),
                status: file.status.clone(),
            })
            .collect::<Vec<_>>();
        self.store
            .replace_project_files(project.id, &records)
            .map_err(storage_error)?;
        self.store
            .set_project_status(project.id, ProjectStatus::ScanReady, None)
            .map_err(storage_error)?;
        let project = self
            .store
            .project(project.id)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project.id))?;
        Ok(ScanReport { project, files })
    }

    /// Imports the persisted scan into an immutable project snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable project identity to import.
    ///
    /// # Returns
    /// Import counts and the promoted snapshot.
    ///
    /// # Errors
    /// Returns a structured validation, storage, or promotion failure.
    pub fn import(&self, project: ProjectId) -> Result<ImportReport, ProjectError> {
        let record = self
            .store
            .project(project)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project))?;
        let files = self.store.project_files(project).map_err(storage_error)?;
        if files.is_empty() {
            return Err(ProjectError {
                code: "scan_required",
                message: "scan the project before importing".to_owned(),
            });
        }
        let old_entities = self
            .store
            .project_entities(project)
            .map_err(storage_error)?;
        let base = record
            .current_snapshot
            .unwrap_or_else(|| Snapshot::empty().id());
        let root = support::project_root(project)?;
        let expected = self.store.read_root(&root);
        let mut revisions = Vec::new();
        let mut changes = old_entities
            .iter()
            .map(|entity| SnapshotChange::Remove(entity.entity))
            .collect::<Vec<_>>();
        let mut ownership = Vec::<ProjectEntityRecord>::new();
        for file in files.iter().filter(|file| file.status == "ready") {
            ownership.extend(import_file(project, file, &mut revisions, &mut changes)?);
        }
        let snapshot = self
            .store
            .apply_revisions_and_promote(root, expected, base, &revisions, &changes)
            .map_err(|error| promotion_error(format!("{error:?}")))?;
        self.store
            .replace_project_entities(project, &ownership)
            .map_err(storage_error)?;
        self.store
            .set_project_status(project, ProjectStatus::Imported, Some(snapshot))
            .map_err(storage_error)?;
        let project_record = self
            .store
            .project(project)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project))?;
        Ok(ImportReport {
            project: project_record,
            files: files.iter().filter(|file| file.status == "ready").count(),
            entities: ownership.len(),
            snapshot,
        })
    }
}

/// Converts a storage failure into a project error.
///
/// # Parameters
/// - `error`: Storage failure.
///
/// # Returns
/// A structured project error.
fn storage_error(error: impl Display) -> ProjectError {
    ProjectError {
        code: "storage_error",
        message: error.to_string(),
    }
}

/// Converts a filesystem failure into a project error.
///
/// # Parameters
/// - `error`: Filesystem failure.
///
/// # Returns
/// A structured project error.
fn filesystem_error(error: &std::io::Error) -> ProjectError {
    ProjectError {
        code: "filesystem_error",
        message: error.to_string(),
    }
}

/// Creates a missing-project error.
///
/// # Parameters
/// - `project`: Missing project identity.
///
/// # Returns
/// A structured project error.
fn missing_project(project: ProjectId) -> ProjectError {
    ProjectError {
        code: "project_not_found",
        message: project.to_string(),
    }
}

/// Creates a snapshot promotion error.
///
/// # Parameters
/// - `message`: Promotion failure detail.
///
/// # Returns
/// A structured project error.
fn promotion_error(message: String) -> ProjectError {
    ProjectError {
        code: "promotion_failed",
        message,
    }
}

/// Creates a source validation error.
///
/// # Parameters
/// - `path`: Source path.
/// - `message`: Validation detail.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// A structured project error.
pub(crate) fn validation_error(path: &str, message: &str) -> ProjectError {
    ProjectError {
        code: "validation_failed",
        message: format!("{path}: {message}"),
    }
}

/// Creates an invalid-input error.
///
/// # Parameters
/// - `message`: Invalid-input detail.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// A structured project error.
pub(crate) fn invalid_input(message: String) -> ProjectError {
    ProjectError {
        code: "invalid_input",
        message,
    }
}
