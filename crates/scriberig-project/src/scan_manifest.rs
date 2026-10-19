//! Persists source scans selected by an explicit project-relative manifest.

use std::collections::BTreeSet;
use std::path::Path;

use scriberig_store_sqlite::{ProjectFileRecord, ProjectRecord, ProjectStatus};

use crate::{ProjectError, ProjectService, ScanReport, errors, support};

impl ProjectService {
    /// Scans only an explicit set of project-relative files.
    ///
    /// This preserves an exact source-selection policy during candidate refresh
    /// while still including candidate-created paths.
    ///
    /// - `name`: Optional display name for a new project.
    /// - `path`: Filesystem project root.
    /// - `paths`: Relative file manifest selected for this scan.
    /// - `crate`: Internal project scan module.
    ///
    /// The registered project and selected deterministic scan files.
    ///
    /// Returns a structured filesystem or storage failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(crate) fn scan_with_paths(
        &self,
        name: Option<String>,
        path: impl AsRef<Path>,
        paths: &BTreeSet<String>,
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
            .map_err(errors::storage_error)?;
        self.persist_scan(&project, Some(paths), false)
    }

    /// Scans and durably stores a registered project's selected files.
    ///
    /// - `project`: Registered project identity and path.
    /// - `paths`: Optional exact relative-path filter.
    /// - `git_tracked`: Whether the selection came from the Git index.
    /// - `super`: Parent project service module.
    ///
    /// Persisted project scan report.
    ///
    /// Returns a structured filesystem or storage failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn persist_scan(
        &self,
        project: &ProjectRecord,
        paths: Option<&BTreeSet<String>>,
        git_tracked: bool,
    ) -> Result<ScanReport, ProjectError> {
        let files = support::scan_directory(&project.path, paths)
            .map_err(|error| errors::filesystem_error(&error))?;
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
            .map_err(errors::storage_error)?;
        self.store
            .set_project_status(
                project.id,
                ProjectStatus::ScanReady,
                project.current_snapshot,
            )
            .map_err(errors::storage_error)?;
        let project = self
            .store
            .project(project.id)
            .map_err(errors::storage_error)?
            .ok_or_else(|| errors::missing_project(project.id))?;
        Ok(ScanReport {
            project,
            files,
            git_tracked,
        })
    }
}
