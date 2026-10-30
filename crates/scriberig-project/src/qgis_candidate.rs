//! Creates exact-base QGIS layer-reference rename candidates.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store::SnapshotStore;

use crate::{
    ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange, ProjectSourceChangeKind,
};

impl ProjectService {
    /// Proposes one exact-base QGIS project layer-order change.
    ///
    /// The operation replaces the complete layer order as one typed semantic
    /// value and uses the shared candidate validation and apply lifecycle.
    ///
    /// # Parameters
    /// - `project`: Project owning the QGIS project file.
    /// - `base_snapshot`: Exact current project snapshot.
    /// - `path`: Exact project-relative QGS/QGZ path.
    /// - `ordered_references`: Complete requested layer-reference permutation.
    ///
    /// # Returns
    /// Durable candidate with a typed `project.layer_order` before/after diff.
    ///
    /// # Errors
    /// Returns an error for stale snapshots, missing sources, or unsupported tree shapes.
    pub fn propose_qgis_layer_order(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        path: &str,
        ordered_references: &[EntityId],
    ) -> Result<ProjectCandidate, ProjectError> {
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(qgis_candidate_error(
                "stale_candidate",
                "requested base is not the active project snapshot",
            ));
        }
        let project_view = self.qgis_project(project, path, base_snapshot)?;
        let before_order = project_view
            .project
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>();
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == path && file.adapter == "qgis")
            .ok_or_else(|| {
                qgis_candidate_error("qgis_source_not_found", "QGIS source is not in the base")
            })?;
        let updated = scriberig_domain_qgis::reorder_project_layers(
            &source.bytes,
            project_view.project.id,
            ordered_references,
        )
        .map_err(|message| qgis_candidate_error("qgis_order_unsupported", &message))?;
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "qgis.reorder_project_layers",
            project_view.project.id,
        )?;
        candidate
            .semantic_changes
            .push(crate::SemanticCandidateChange {
                target: project_view.project.id,
                property: "project.layer_order".to_owned(),
                before: serde_json::json!(before_order),
                after: serde_json::json!(ordered_references),
            });
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }

    /// Proposes one exact-base QGIS project layer rename.
    ///
    /// Only the display name of the selected layer reference is changed. The
    /// domain validator proves the staged bytes are exactly the safe writer's
    /// output before the shared candidate can be applied.
    ///
    /// # Parameters
    /// - `project`: Project owning the QGIS project.
    /// - `base_snapshot`: Exact current project snapshot.
    /// - `reference`: Stable QGIS layer-reference identity.
    /// - `new_name`: New authored project display name.
    ///
    /// # Returns
    /// Durable candidate for validation and shared apply.
    ///
    /// # Errors
    /// Returns an error for stale, unowned, unresolved, malformed, or unsupported sources.
    pub fn propose_qgis_layer_rename(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        reference: EntityId,
        new_name: &str,
    ) -> Result<ProjectCandidate, ProjectError> {
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(qgis_candidate_error(
                "stale_candidate",
                "requested base is not the active project snapshot",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, reference)
            .map_err(crate::errors::storage_error)?
            .filter(|entity| entity.kind == "qgis.layer_reference")
            .ok_or_else(|| {
                qgis_candidate_error(
                    "qgis_layer_reference_not_found",
                    "target is not an owned QGIS layer reference",
                )
            })?;
        self.store
            .resolve(base_snapshot, reference)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                qgis_candidate_error(
                    "qgis_layer_reference_not_in_snapshot",
                    "layer reference is absent from the requested snapshot",
                )
            })?;
        let project_view = self.qgis_project(project, &ownership.relative_path, base_snapshot)?;
        let prior_name = project_view
            .project
            .layers
            .iter()
            .find(|layer| layer.id == reference)
            .map(|layer| layer.name.clone())
            .ok_or_else(|| {
                qgis_candidate_error(
                    "qgis_layer_reference_mismatch",
                    "owned semantic reference is absent from its exact project revision",
                )
            })?;
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == ownership.relative_path && file.adapter == "qgis")
            .ok_or_else(|| {
                qgis_candidate_error("qgis_source_not_found", "QGIS source is not in the base")
            })?;
        let updated = scriberig_domain_qgis::rename_layer_reference(
            &source.bytes,
            project_view.project.id,
            reference,
            new_name,
        )
        .map_err(|message| qgis_candidate_error("qgis_rename_unsupported", &message))?;
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "qgis.rename_layer_reference",
            reference,
        )?;
        candidate
            .semantic_changes
            .push(crate::SemanticCandidateChange {
                target: reference,
                property: "layer_reference.name".to_owned(),
                before: serde_json::json!(prior_name),
                after: serde_json::json!(new_name),
            });
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }
}

/// Creates a safe QGIS candidate error without source bytes.
///
/// # Parameters
/// - `code`: Stable error category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// Structured project error for application and protocol surfaces.
fn qgis_candidate_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
