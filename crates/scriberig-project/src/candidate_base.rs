//! Reconstructs exact project source manifests from immutable snapshots.

use scriberig_core::{ProjectId, SnapshotId};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::{ProjectFileRecord, SqliteStore};

use crate::ProjectError;
use crate::support::adapter_for;

/// Loads project source artifacts from one exact immutable snapshot.
///
/// - `store`: Shared project object and ownership store.
/// - `project`: Project whose sources are reconstructed.
/// - `snapshot`: Exact source snapshot.
/// - `super`: Parent candidate module visibility.
///
/// Deterministically ordered source file records represented by the snapshot.
///
/// Returns an error when source ownership or immutable revisions are inconsistent.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn project_files_at(
    store: &SqliteStore,
    project: ProjectId,
    snapshot: SnapshotId,
) -> Result<Vec<ProjectFileRecord>, ProjectError> {
    let entities = store
        .project_entities(project)
        .map_err(crate::errors::storage_error)?;
    let mut files = Vec::new();
    for entity in entities
        .into_iter()
        .filter(|entity| entity.kind == "source")
    {
        let revision_id = store
            .resolve(snapshot, entity.entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| invalid_source(&entity.relative_path))?;
        let revision = store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| invalid_source(&entity.relative_path))?;
        if revision.object_type().as_str() != "project.source" {
            return Err(invalid_source(&entity.relative_path));
        }
        let bytes = revision.payload().to_vec();
        let adapter = adapter_for(std::path::Path::new(&entity.relative_path), &bytes);
        let grouping_key = crate::support::logical_grouping_key(&entity.relative_path);
        files.push(ProjectFileRecord {
            project_id: project,
            relative_path: entity.relative_path,
            entity: entity.entity,
            digest: *blake3::hash(&bytes).as_bytes(),
            bytes,
            adapter: adapter.to_owned(),
            status: if adapter == "unsupported" {
                "unsupported"
            } else {
                "ready"
            }
            .to_owned(),
            grouping_key,
        });
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

/// Creates a safe corruption error for an unavailable source artifact.
///
/// - `path`: Project-relative source path.
///
/// Structured project source error.
/// # Returns
/// The operation result.
fn invalid_source(path: &str) -> ProjectError {
    ProjectError {
        code: "project_snapshot_corrupt",
        message: format!("snapshot source is unavailable: {path}"),
        diagnostics: Vec::new(),
    }
}
