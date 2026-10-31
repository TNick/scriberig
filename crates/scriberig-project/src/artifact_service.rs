//! Exposes logical artifact summaries enriched with persisted adapter capability.

use scriberig_core::{EntityId, ProjectId};

use crate::{
    ArtifactCapability, ArtifactGroupingDiagnostic, ArtifactManifest, ImportDiagnostic,
    ProjectError, ProjectService, artifact,
};

#[path = "artifact_service/synchronization.rs"]
mod synchronization;
pub(super) use synchronization::synchronization_for_artifact;
pub use synchronization::{ArtifactSynchronizationState, ArtifactSynchronizationStatus};

#[cfg(test)]
#[path = "artifact_service/tests.rs"]
mod tests;

impl ProjectService {
    /// Checks live artifact members against their exact persisted source baseline.
    ///
    /// - `project`: Project owning the artifacts.
    /// - `artifacts`: Bounded artifact summaries to inspect.
    ///
    /// One synchronization record per artifact, in input order.
    ///
    /// Returns an error when project metadata or persisted artifact evidence is unavailable.
    /// # Returns
    /// Live member status with exact baseline and observed digests.
    /// # Errors
    /// Returns a project error when project metadata cannot be loaded.
    pub fn artifact_synchronization(
        &self,
        project: ProjectId,
        artifacts: &[ArtifactManifest],
    ) -> Result<Vec<ArtifactSynchronizationStatus>, ProjectError> {
        let record = self
            .store
            .project(project)
            .map_err(crate::errors::storage_error)?
            .ok_or_else(|| ProjectError {
                code: "project_not_found",
                message: "artifact project is unavailable".to_owned(),
                diagnostics: Vec::new(),
            })?;
        let root = record.path;
        Ok(artifacts
            .iter()
            .map(|artifact| synchronization_for_artifact(&root, artifact))
            .collect())
    }

    /// Reads a bounded exact-evidence member page for one artifact.
    ///
    /// # Parameters
    /// - `project`: Project whose persisted scan owns the artifact.
    /// - `artifact`: Stable semantic artifact entity identity.
    /// - `offset`: Zero-based member offset.
    /// - `limit`: Maximum number of member descriptors to return.
    ///
    /// # Returns
    /// Deterministic page of physical and package-internal member evidence.
    ///
    /// # Errors
    /// Returns an error when the artifact is absent or page bounds are invalid.
    pub fn artifact_members(
        &self,
        project: ProjectId,
        artifact: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<crate::ArtifactMemberPage, ProjectError> {
        if !(1..=500).contains(&limit) {
            return Err(ProjectError {
                code: "artifact_member_page_limit_invalid",
                message: "artifact member page limit must be between one and five hundred".into(),
                diagnostics: Vec::new(),
            });
        }
        let manifest = self
            .artifacts(project)?
            .into_iter()
            .find(|manifest| manifest.id.entity() == artifact)
            .ok_or_else(|| ProjectError {
                code: "artifact_not_found",
                message: "artifact is not present in the persisted project catalog".into(),
                diagnostics: Vec::new(),
            })?;
        if offset > manifest.members.len() {
            return Err(ProjectError {
                code: "artifact_member_offset_invalid",
                message: "artifact member offset exceeds the available member count".into(),
                diagnostics: Vec::new(),
            });
        }
        let total = manifest.members.len();
        let members = manifest
            .members
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        let next_offset = (offset + members.len() < total).then_some(offset + members.len());
        Ok(crate::ArtifactMemberPage {
            artifact: manifest.id,
            members,
            total,
            next_offset,
        })
    }

    /// Resolves persisted artifact edit capability for one exact source path.
    ///
    /// # Parameters
    /// - `project`: Project whose persisted artifact catalog is consulted.
    /// - `path`: Project-relative source path associated with a semantic root.
    ///
    /// # Returns
    /// The manifest capability for the artifact containing the path, when present.
    ///
    /// # Errors
    /// Returns a project error when persisted artifact data cannot be read.
    pub(crate) fn artifact_capability_for_path(
        &self,
        project: ProjectId,
        path: &str,
    ) -> Result<Option<ArtifactCapability>, ProjectError> {
        Ok(self
            .artifacts(project)?
            .into_iter()
            .find(|artifact| {
                artifact
                    .members
                    .iter()
                    .any(|member| member.project_path.as_deref() == Some(path))
            })
            .map(|artifact| artifact.fidelity.capability))
    }

    /// Resolves persisted artifact fidelity for one exact source path.
    ///
    /// # Parameters
    /// - `project`: Project whose persisted artifact catalog is consulted.
    /// - `path`: Project-relative source path associated with a semantic root.
    ///
    /// # Returns
    /// The full fidelity record for the artifact containing the path, when present.
    ///
    /// # Errors
    /// Returns a project error when persisted artifact data cannot be read.
    pub(crate) fn artifact_fidelity_for_path(
        &self,
        project: ProjectId,
        path: &str,
    ) -> Result<Option<crate::ArtifactFidelity>, ProjectError> {
        Ok(self
            .artifacts(project)?
            .into_iter()
            .find(|artifact| {
                artifact
                    .members
                    .iter()
                    .any(|member| member.project_path.as_deref() == Some(path))
            })
            .map(|artifact| artifact.fidelity))
    }

    /// Requires the shared artifact fidelity to permit an exact source edit.
    ///
    /// The source digest must match the persisted artifact member. This binds
    /// candidate creation to the same bytes used for capability discovery.
    ///
    /// # Parameters
    /// - `project`: Project owning the source member.
    /// - `relative_path`: Exact project-relative source path.
    /// - `digest`: Exact source digest at the candidate base snapshot.
    ///
    /// # Returns
    /// Unit when the exact artifact source is editable.
    ///
    /// # Errors
    /// Returns an error when the source capability is missing, stale, or read-only.
    pub(crate) fn require_editable_source(
        &self,
        project: ProjectId,
        relative_path: &str,
        digest: [u8; 32],
    ) -> Result<(), ProjectError> {
        let manifests = self.artifacts(project)?;
        let manifest = manifests
            .iter()
            .find(|manifest| {
                manifest.members.iter().any(|member| {
                    member.project_path.as_deref() == Some(relative_path) && member.digest == digest
                })
            })
            .ok_or_else(|| ProjectError {
                code: "artifact_fidelity_stale",
                message: "artifact capability does not match the exact source snapshot".to_owned(),
                diagnostics: Vec::new(),
            })?;
        if matches!(
            manifest.fidelity.capability,
            ArtifactCapability::LosslessEditable
                | ArtifactCapability::LossAwareEditable
                | ArtifactCapability::EditableWithOpaquePreservation
        ) {
            return Ok(());
        }
        let diagnostic = manifest
            .fidelity
            .diagnostic
            .clone()
            .unwrap_or_else(|| "artifact adapter does not permit semantic editing".to_owned());
        Err(ProjectError {
            code: "artifact_read_only",
            message: diagnostic.clone(),
            diagnostics: vec![ImportDiagnostic {
                entity: None,
                path: relative_path.to_owned(),
                adapter: manifest.source.adapter.clone(),
                stage: "fidelity".to_owned(),
                code: "artifact_read_only".to_owned(),
                message: diagnostic,
                retryable: false,
            }],
        })
    }

    /// Lists logical artifacts derived from the exact persisted project scan.
    ///
    /// This operation never rescans the filesystem. Adapter fidelity is
    /// resolved from source bytes retained by project onboarding.
    ///
    /// # Parameters
    /// - `project`: Project whose persisted scan is summarized.
    ///
    /// # Returns
    /// Deterministically ordered artifact manifests.
    ///
    /// # Errors
    /// Returns a storage error when persisted source evidence cannot be read.
    pub fn artifacts(&self, project: ProjectId) -> Result<Vec<ArtifactManifest>, ProjectError> {
        let files = self
            .store
            .project_file_descriptors(project)
            .map_err(crate::errors::storage_error)?;
        let sources = self
            .store
            .project_files(project)
            .map_err(crate::errors::storage_error)?;
        let mut manifests = artifact::group_persisted_files(project, &files);
        artifact::append_container_member_evidence(&mut manifests, &sources);
        artifact::resolve_adapter_fidelity(&mut manifests, &sources, &self.cad_provider_registry);
        Ok(manifests)
    }

    /// Reports ambiguous grouping decisions from the persisted scan.
    ///
    /// This operation uses the same exact scan evidence as [`Self::artifacts`]
    /// and never consults the current filesystem.
    ///
    /// # Parameters
    /// - `project`: Project whose persisted scan is checked.
    ///
    /// # Returns
    /// Deterministically ordered grouping diagnostics.
    ///
    /// # Errors
    /// Returns a storage error when persisted source evidence cannot be read.
    pub fn artifact_grouping_diagnostics(
        &self,
        project: ProjectId,
    ) -> Result<Vec<ArtifactGroupingDiagnostic>, ProjectError> {
        let files = self
            .store
            .project_file_descriptors(project)
            .map_err(crate::errors::storage_error)?;
        Ok(artifact::group_persisted_files_with_diagnostics(project, &files).1)
    }
}
