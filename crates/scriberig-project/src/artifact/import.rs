//! Adds imported logical artifact revisions alongside domain semantic entities.

use scriberig_core::ProjectId;
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use super::{ArtifactId, append_container_member_evidence, group_persisted_files};
use crate::{ProjectError, support};

/// Appends one stable semantic artifact entity for each ready persisted group.
///
/// # Parameters
/// - `project`: Project owning the exact source records.
/// - `files`: Persisted source records captured for this import.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Errors
/// Returns an error when a manifest cannot be encoded or its revision is invalid.
///
/// # Returns
/// Unit after artifact revisions and ownership records have been appended.
pub(crate) fn append_artifact_revisions(
    project: ProjectId,
    files: &[ProjectFileRecord],
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let ready_files = files
        .iter()
        .filter(|file| file.status == "ready")
        .collect::<Vec<_>>();
    let descriptors = ready_files
        .iter()
        .map(|file| scriberig_store_sqlite::ProjectFileDescriptor {
            project_id: file.project_id,
            relative_path: file.relative_path.clone(),
            entity: file.entity,
            digest: file.digest,
            adapter: file.adapter.clone(),
            status: file.status.clone(),
            grouping_key: file.grouping_key.clone(),
        })
        .collect::<Vec<_>>();
    let mut manifests = group_persisted_files(project, &descriptors);
    append_container_member_evidence(&mut manifests, files);
    for manifest in manifests {
        let source = ready_files
            .iter()
            .find(|file| ArtifactId::from_source_entity(file.entity) == manifest.id)
            .ok_or_else(|| ProjectError {
                code: "artifact_source_missing",
                message: "artifact identity source is absent from import".to_owned(),
                diagnostics: Vec::new(),
            })?;
        let payload = manifest.canonical_bytes().map_err(|error| ProjectError {
            code: "artifact_manifest_encoding_failed",
            message: format!("artifact manifest encoding failed: {error}"),
            diagnostics: Vec::new(),
        })?;
        let revision = support::revision("external_artifact", payload)?;
        changes.push(SnapshotChange::Set(manifest.id.entity(), revision.id()));
        revisions.push(revision);
        ownership.push(ProjectEntityRecord {
            project_id: project,
            relative_path: source.relative_path.clone(),
            entity: manifest.id.entity(),
            kind: "external_artifact".to_owned(),
            name: Some(manifest.logical_key),
        });
    }
    Ok(())
}
