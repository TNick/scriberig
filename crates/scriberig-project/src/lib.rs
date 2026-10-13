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
use support::import_file;
#[path = "candidate.rs"]
mod candidate;
#[path = "manifest.rs"]
mod manifest;
#[path = "project_views.rs"]
mod project_views;
pub use candidate::{
    CandidateApplyReport, CandidateValidation, ProjectCandidate, ProjectSourceChange,
    ProjectSourceChangeKind, ProjectSourceEffect, ProjectSourceEffectKind,
};
pub use manifest::{ProjectManifestEntry, ProjectManifestPage, ProjectManifestSummary};
pub use project_views::{ProjectSourceProjection, ProjectSymbol};
#[path = "errors.rs"]
mod errors;
use errors::{filesystem_error, git_error, missing_project, promotion_error, storage_error};

/// Shared project onboarding service used by CLI, HTTP, and TUI surfaces.
#[derive(Clone, Debug)]
pub struct ProjectService {
    /// Shared durable SQLite catalog and object store.
    store: Arc<SqliteStore>,
}

/// Selects which filesystem files a project scan persists.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScanOptions {
    /// Restricts the manifest to files tracked by the Git index.
    pub git_tracked: bool,
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
    /// Whether the manifest was restricted to Git-indexed files.
    pub git_tracked: bool,
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

/// Resolves the exact active semantic workspace for one imported project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectWorkspace {
    /// Canonical project catalog record.
    pub project: ProjectRecord,
    /// Current immutable project snapshot.
    pub snapshot: SnapshotId,
    /// Semantic source root used to begin navigation.
    pub root_entity: scriberig_core::EntityId,
}

/// Describes one recoverable or terminal import diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportDiagnostic {
    /// Project-relative file path.
    pub path: String,
    /// Adapter that produced the diagnostic.
    pub adapter: String,
    /// Import stage that failed.
    pub stage: String,
    /// Stable diagnostic category.
    pub code: String,
    /// Safe diagnostic message.
    pub message: String,
    /// Whether editing the source and retrying may succeed.
    pub retryable: bool,
}

/// Reports a project onboarding failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectError {
    /// Stable failure category.
    pub code: &'static str,
    /// Safe human-readable failure detail.
    pub message: String,
    /// Structured diagnostics suitable for agents and product surfaces.
    pub diagnostics: Vec<ImportDiagnostic>,
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
        self.scan_with_options(name, path, ScanOptions::default())
    }

    /// Registers and persists a scan using an explicit file-selection policy.
    ///
    /// # Parameters
    /// - `name`: Optional display name for a new project.
    /// - `path`: Filesystem project path.
    /// - `options`: File-selection policy for the persisted manifest.
    ///
    /// # Returns
    /// The registered project and selected deterministic scan files.
    ///
    /// # Errors
    /// Returns a structured filesystem, Git, or storage failure.
    pub fn scan_with_options(
        &self,
        name: Option<String>,
        path: impl AsRef<Path>,
        options: ScanOptions,
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
        let tracked = if options.git_tracked {
            Some(support::git_tracked_paths(&project.path).map_err(|error| git_error(&error))?)
        } else {
            None
        };
        let files = support::scan_directory(&project.path, tracked.as_ref())
            .map_err(|error| filesystem_error(&error))?;
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
            .set_project_status(
                project.id,
                ProjectStatus::ScanReady,
                project.current_snapshot,
            )
            .map_err(storage_error)?;
        let project = self
            .store
            .project(project.id)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project.id))?;
        Ok(ScanReport {
            project,
            files,
            git_tracked: options.git_tracked,
        })
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
        let files = self.store.project_files(project).map_err(storage_error)?;
        self.import_file_records(project, &files)
    }

    /// Imports only when the latest persisted manifest matches its exact digest.
    ///
    /// # Parameters
    /// - `project`: Project owning the persisted scan.
    /// - `manifest_id`: Digest returned by the scan manifest summary.
    ///
    /// # Returns
    /// Import counts and the promoted immutable snapshot.
    ///
    /// # Errors
    /// Returns a structured error when the project is missing, the manifest is
    /// stale, or importing the captured manifest fails.
    pub fn import_manifest(
        &self,
        project: ProjectId,
        manifest_id: &str,
    ) -> Result<ImportReport, ProjectError> {
        let files = self.store.project_files(project).map_err(storage_error)?;
        if manifest::digest(project, &files) != manifest_id {
            return Err(ProjectError {
                code: "manifest_changed",
                message: "persisted project manifest no longer matches the reviewed scan".into(),
                diagnostics: Vec::new(),
            });
        }
        self.import_file_records(project, &files)
    }

    /// Imports the exact file records captured by one persisted manifest read.
    ///
    /// # Parameters
    /// - `project`: Project being imported.
    /// - `files`: Immutable records captured from one persisted manifest read.
    ///
    /// # Returns
    /// Import counts and the promoted immutable snapshot.
    ///
    /// # Errors
    /// Returns a structured validation, storage, or promotion failure.
    fn import_file_records(
        &self,
        project: ProjectId,
        files: &[ProjectFileRecord],
    ) -> Result<ImportReport, ProjectError> {
        let record = self
            .store
            .project(project)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project))?;
        if files.is_empty() {
            return Err(ProjectError {
                code: "scan_required",
                message: "scan the project before importing".to_owned(),
                diagnostics: Vec::new(),
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
            .apply_project_import(
                project, root, expected, base, &revisions, &changes, &ownership,
            )
            .map_err(|error| promotion_error(format!("{error:?}")))?;
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

    /// Resolves an imported project's current snapshot and semantic root.
    ///
    /// # Parameters
    /// - `project`: Project whose active workspace should be resolved.
    ///
    /// # Returns
    /// The project record, current snapshot, and semantic root entity.
    ///
    /// # Errors
    /// Returns a structured error when the project is missing or not imported.
    pub fn workspace(&self, project: ProjectId) -> Result<ProjectWorkspace, ProjectError> {
        let record = self
            .store
            .project(project)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project))?;
        let snapshot = record.current_snapshot.ok_or_else(|| ProjectError {
            code: "project_not_imported",
            message: "project has no current semantic snapshot".to_owned(),
            diagnostics: Vec::new(),
        })?;
        let root_entity = self
            .store
            .project_source_root(project)
            .map_err(storage_error)?;
        let root_entity = root_entity
            .map(|entity| entity.entity)
            .ok_or_else(|| ProjectError {
                code: "project_root_unavailable",
                message: "project has no semantic source root".to_owned(),
                diagnostics: Vec::new(),
            })?;
        Ok(ProjectWorkspace {
            project: record,
            snapshot,
            root_entity,
        })
    }
}
