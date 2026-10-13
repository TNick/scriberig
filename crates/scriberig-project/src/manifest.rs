//! Exposes bounded, digest-bound views of persisted project scan manifests.

use scriberig_core::ProjectId;

use crate::{
    ProjectError, ProjectService,
    errors::{missing_project, storage_error},
};

/// Identifies one exact persisted project scan manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifestSummary {
    /// Project owning the manifest.
    pub project: ProjectId,
    /// Human-readable project name.
    pub name: String,
    /// Canonical checkout path registered by the local project service.
    pub path: String,
    /// Stable digest identifying the exact manifest contents.
    pub manifest_id: String,
    /// Number of persisted files in the manifest.
    pub file_count: usize,
    /// Number of files classified as ready for import.
    pub ready_count: usize,
    /// Number of files excluded or unsupported by adapters.
    pub other_count: usize,
}

/// Describes one bounded manifest entry without exposing its source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifestEntry {
    /// Project-relative path of the captured file.
    pub path: String,
    /// Digest of exact source bytes captured at scan time.
    pub source_digest: String,
    /// Adapter selected by deterministic classification.
    pub adapter: String,
    /// Stable import classification.
    pub status: String,
}

/// Returns one page from a digest-bound manifest listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifestPage {
    /// Project owning the manifest.
    pub project: ProjectId,
    /// Exact manifest digest shared with the summary.
    pub manifest_id: String,
    /// Bounded manifest entries.
    pub entries: Vec<ProjectManifestEntry>,
    /// Offset for the next page, when more entries remain.
    pub next_offset: Option<usize>,
}

/// Computes the stable identity of one persisted manifest.
///
/// # Parameters
/// - `super`: Parent project module.
/// - `project`: Project whose manifest is hashed.
/// - `files`: Deterministically ordered persisted source records.
///
/// # Returns
/// Lowercase hexadecimal BLAKE3 digest over project, paths, source digests,
/// adapter identifiers, and classifications.
pub(super) fn digest(
    project: ProjectId,
    files: &[scriberig_store_sqlite::ProjectFileRecord],
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(project.as_bytes());
    for file in files {
        hasher.update(&(file.relative_path.len() as u64).to_le_bytes());
        hasher.update(file.relative_path.as_bytes());
        hasher.update(&file.digest);
        hasher.update(&(file.adapter.len() as u64).to_le_bytes());
        hasher.update(file.adapter.as_bytes());
        hasher.update(&(file.status.len() as u64).to_le_bytes());
        hasher.update(file.status.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

impl ProjectService {
    /// Summarizes one persisted project manifest without returning file bodies.
    ///
    /// # Parameters
    /// - `self`: Shared project scan service.
    /// - `project`: Project whose current manifest is requested.
    ///
    /// # Returns
    /// Stable project metadata, manifest identity, and classification counts.
    ///
    /// # Errors
    /// Returns a structured error when the project or manifest cannot be read.
    pub fn manifest_summary(
        &self,
        project: ProjectId,
    ) -> Result<ProjectManifestSummary, ProjectError> {
        let record = self
            .store
            .project(project)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project))?;
        let files = self.store.project_files(project).map_err(storage_error)?;
        Ok(ProjectManifestSummary {
            project,
            name: record.name,
            path: record.path.to_string_lossy().into_owned(),
            manifest_id: digest(project, &files),
            file_count: files.len(),
            ready_count: files.iter().filter(|file| file.status == "ready").count(),
            other_count: files.iter().filter(|file| file.status != "ready").count(),
        })
    }

    /// Reads a bounded page from the exact current persisted manifest.
    ///
    /// # Parameters
    /// - `self`: Shared project scan service.
    /// - `project`: Project owning the manifest.
    /// - `manifest_id`: Digest returned by `manifest_summary`.
    /// - `offset`: Zero-based entry offset.
    /// - `limit`: Requested page size, capped at one hundred entries.
    ///
    /// # Returns
    /// A bounded listing and the next offset when entries remain.
    ///
    /// # Errors
    /// Returns an error for a missing project, stale digest, or invalid page size.
    pub fn manifest_page(
        &self,
        project: ProjectId,
        manifest_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<ProjectManifestPage, ProjectError> {
        if limit == 0 {
            return Err(invalid_page());
        }
        let files = self.store.project_files(project).map_err(storage_error)?;
        if digest(project, &files) != manifest_id {
            return Err(stale_manifest());
        }
        if self
            .store
            .project(project)
            .map_err(storage_error)?
            .is_none()
        {
            return Err(missing_project(project));
        }
        let end = offset.saturating_add(limit.min(100)).min(files.len());
        let entries = files
            .get(offset.min(files.len())..end)
            .unwrap_or_default()
            .iter()
            .map(|file| ProjectManifestEntry {
                path: file.relative_path.clone(),
                source_digest: blake3::Hash::from_bytes(file.digest).to_hex().to_string(),
                adapter: file.adapter.clone(),
                status: file.status.clone(),
            })
            .collect();
        Ok(ProjectManifestPage {
            project,
            manifest_id: manifest_id.to_owned(),
            entries,
            next_offset: (end < files.len()).then_some(end),
        })
    }
}

/// Creates a structured failure for an invalid manifest page size.
///
/// # Returns
/// An invalid manifest page error.
fn invalid_page() -> ProjectError {
    ProjectError {
        code: "invalid_manifest_page",
        message: "manifest page size must be greater than zero".into(),
        diagnostics: Vec::new(),
    }
}

/// Creates a structured failure when a caller's manifest reference is stale.
///
/// # Returns
/// A stale manifest error.
fn stale_manifest() -> ProjectError {
    ProjectError {
        code: "manifest_changed",
        message: "persisted project manifest no longer matches the reviewed scan".into(),
        diagnostics: Vec::new(),
    }
}
