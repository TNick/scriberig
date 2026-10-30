//! Persists source scans selected by an explicit project-relative manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use scriberig_core::EntityId;
use scriberig_store_sqlite::{
    ProjectFileDescriptor, ProjectFileRecord, ProjectRecord, ProjectStatus,
};

use crate::{ProjectError, ProjectFileRename, ProjectService, ScanReport, errors, support};

impl ProjectService {
    /// Scans only an explicit set of project-relative files.
    ///
    /// This preserves an exact source-selection policy during candidate refresh
    /// while still including candidate-created paths.
    ///
    /// # Parameters
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
        self.persist_scan(&project, Some(paths), false, &[])
    }

    /// Scans and durably stores a registered project's selected files.
    ///
    /// # Parameters
    /// - `project`: Registered project identity and path.
    /// - `paths`: Optional exact relative-path filter.
    /// - `git_tracked`: Whether the selection came from the Git index.
    /// - `renames`: Explicit previous-path to current-path observations.
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
        renames: &[ProjectFileRename],
    ) -> Result<ScanReport, ProjectError> {
        let files = support::scan_directory(&project.path, paths)
            .map_err(|error| errors::filesystem_error(&error))?;
        let previous = self
            .store
            .project_file_descriptors(project.id)
            .map_err(errors::storage_error)?;
        let source_entities = source_entities_for_scan(&files, &previous, renames)?;
        let records = files
            .iter()
            .zip(source_entities)
            .map(|(file, entity)| ProjectFileRecord {
                project_id: project.id,
                relative_path: file.relative_path.clone(),
                entity,
                digest: *blake3::hash(&file.bytes).as_bytes(),
                bytes: file.bytes.clone(),
                adapter: file.adapter.clone(),
                status: file.status.clone(),
                grouping_key: file.grouping_key.clone(),
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

/// Preserves source identity for unchanged paths and evidence-backed renames.
///
/// Explicit rename evidence handles changed content. Automatic rename matching
/// requires one removed and one added file with the same exact digest, adapter,
/// and classification. Ambiguous matches never guess continuity.
///
/// # Parameters
/// - `files`: Newly scanned exact source files.
/// - `previous`: Persisted descriptors from the preceding scan.
/// - `renames`: Explicit previous-path to current-path observations.
///
/// # Returns
/// One stable source entity identity per new file, in input order.
///
/// # Errors
/// Returns a structured error for invalid explicit rename evidence.
fn source_entities_for_scan(
    files: &[crate::ScanFile],
    previous: &[ProjectFileDescriptor],
    renames: &[ProjectFileRename],
) -> Result<Vec<EntityId>, ProjectError> {
    let previous_by_path = previous
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let new_paths = files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<BTreeSet<_>>();
    let mut explicit_sources = BTreeSet::new();
    let mut explicit_targets = BTreeMap::<String, EntityId>::new();
    for rename in renames {
        if !safe_project_relative_path(&rename.from_path)
            || !safe_project_relative_path(&rename.to_path)
            || rename.from_path == rename.to_path
            || !explicit_sources.insert(rename.from_path.as_str())
            || explicit_targets.contains_key(&rename.to_path)
        {
            return Err(invalid_rename("rename paths must be safe and unique"));
        }
        let Some(source) = previous_by_path.get(rename.from_path.as_str()) else {
            return Err(invalid_rename(
                "rename source is absent from the prior scan",
            ));
        };
        if new_paths.contains(rename.from_path.as_str())
            || previous_by_path.contains_key(rename.to_path.as_str())
        {
            return Err(invalid_rename(
                "rename source must disappear and target must be newly added",
            ));
        }
        let Some(target) = files
            .iter()
            .find(|file| file.relative_path == rename.to_path)
        else {
            return Err(invalid_rename(
                "rename target is absent from the current scan",
            ));
        };
        if source.adapter != target.adapter {
            return Err(invalid_rename(
                "rename source and target use different adapters",
            ));
        }
        explicit_targets.insert(rename.to_path.clone(), source.entity);
    }
    let mut old_renames =
        BTreeMap::<(String, [u8; 32], String), Vec<&ProjectFileDescriptor>>::new();
    for old in previous.iter().filter(|old| {
        !new_paths.contains(old.relative_path.as_str())
            && !explicit_sources.contains(old.relative_path.as_str())
    }) {
        old_renames
            .entry((old.adapter.clone(), old.digest, old.status.clone()))
            .or_default()
            .push(old);
    }
    let mut new_renames = BTreeMap::<(String, [u8; 32], String), Vec<usize>>::new();
    for (index, file) in files.iter().enumerate() {
        if previous_by_path.contains_key(file.relative_path.as_str())
            || explicit_targets.contains_key(&file.relative_path)
        {
            continue;
        }
        let signature = (
            file.adapter.clone(),
            *blake3::hash(&file.bytes).as_bytes(),
            file.status.clone(),
        );
        new_renames.entry(signature).or_default().push(index);
    }
    let mut entities = files
        .iter()
        .map(|file| {
            explicit_targets
                .get(&file.relative_path)
                .copied()
                .or_else(|| {
                    previous_by_path
                        .get(file.relative_path.as_str())
                        .map(|old| old.entity)
                })
                .unwrap_or_else(EntityId::new)
        })
        .collect::<Vec<_>>();
    for (signature, new_indexes) in new_renames {
        let Some(old_files) = old_renames.get(&signature) else {
            continue;
        };
        if old_files.len() == 1 && new_indexes.len() == 1 {
            entities[new_indexes[0]] = old_files[0].entity;
        }
    }
    Ok(entities)
}

/// Checks whether a rename-map path is a canonical project-relative POSIX path.
///
/// # Parameters
/// - `path`: Relative path supplied by the rename observer.
///
/// # Returns
/// True when the path has only normal components and no platform separators.
fn safe_project_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

/// Creates a safe diagnostic for rejected project file rename evidence.
///
/// # Parameters
/// - `message`: Explanation that does not include source file contents.
///
/// # Returns
/// A structured project error identifying the invalid rename request.
fn invalid_rename(message: &str) -> ProjectError {
    ProjectError {
        code: "invalid_project_file_rename",
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
