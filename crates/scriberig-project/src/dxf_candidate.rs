//! Creates exact-base semantic `DXF` geometry candidates through shared changes.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_cad::{CadEntity, Coordinate, DrawingEntity, Layer};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::dxf_candidate_errors::{dxf_candidate_entity_error, dxf_candidate_error};
use crate::{
    ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange, ProjectSourceChangeKind,
};

impl ProjectService {
    /// Proposes replacement endpoints for one imported `DXF` line.
    ///
    /// The candidate is tied to the exact active snapshot and stores rendered
    /// source bytes through the shared project change path. The live file is
    /// never changed by this operation.
    ///
    /// # Parameters
    /// - `project`: Project owning the `DXF` source.
    /// - `base_snapshot`: Exact active snapshot used to resolve the line.
    /// - `entity`: Stable semantic identity of the line.
    /// - `start`: New exact decimal start coordinate.
    /// - `end`: New exact decimal end coordinate.
    ///
    /// # Returns
    /// Durable source candidate for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns an error for stale, missing, non-line, unsupported, or invalid targets.
    pub fn propose_dxf_line_geometry(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        entity: EntityId,
        start: &Coordinate,
        end: &Coordinate,
    ) -> Result<ProjectCandidate, ProjectError> {
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(dxf_candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, entity)
            .map_err(crate::errors::storage_error)?
            .filter(|owned| owned.kind == "cad.entity")
            .ok_or_else(|| {
                dxf_candidate_error("cad_entity_not_found", "target is not an owned CAD entity")
            })?;
        let revision_id = self
            .store
            .resolve(base_snapshot, entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                dxf_candidate_error(
                    "cad_entity_not_in_snapshot",
                    "CAD entity is absent from base",
                )
            })?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                dxf_candidate_error(
                    "cad_entity_revision_missing",
                    "CAD entity revision is unavailable",
                )
            })?;
        let semantic: DrawingEntity = serde_json::from_slice(revision.payload()).map_err(|_| {
            dxf_candidate_error("cad_entity_invalid", "CAD entity payload is invalid")
        })?;
        if !matches!(&semantic.geometry, CadEntity::Line { .. }) {
            return Err(dxf_candidate_error(
                "dxf_target_not_line",
                "target geometry is not a line",
            ));
        }
        let handle =
            self.dxf_external_handle(project, base_snapshot, &ownership.relative_path, entity)?;
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == ownership.relative_path && file.adapter == "dxf")
            .ok_or_else(|| {
                dxf_candidate_error("dxf_source_not_found", "CAD entity source is not DXF")
            })?;
        self.require_editable_source(project, &source.relative_path, source.digest)?;
        let updated =
            scriberig_domain_dxf::replace_line_geometry(&source.bytes, &handle, start, end)
                .map_err(|error| {
                    dxf_candidate_entity_error(
                        error.code,
                        &error.message,
                        &source.relative_path,
                        entity,
                    )
                })?;
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "cad.replace_line_geometry",
            entity,
        )?;
        self.persist_cad_semantic_change(
            &mut candidate,
            entity,
            "entity.geometry",
            serde_json::to_value(&semantic.geometry).map_err(|_| {
                dxf_candidate_error("cad_diff_invalid", "prior geometry is invalid")
            })?,
            serde_json::to_value(CadEntity::Line {
                start: start.clone(),
                end: end.clone(),
            })
            .map_err(|_| dxf_candidate_error("cad_diff_invalid", "proposed geometry is invalid"))?,
        )?;
        Ok(candidate)
    }

    /// Proposes moving one imported DXF entity to an existing drawing layer.
    ///
    /// The target and destination layer must belong to the same exact drawing
    /// source. The candidate stores rendered bytes through the shared project
    /// change path and never writes the live file.
    ///
    /// # Parameters
    /// - `project`: Project owning the DXF source.
    /// - `base_snapshot`: Exact active snapshot used to resolve both entities.
    /// - `entity`: Stable semantic identity of the CAD entity to move.
    /// - `destination_layer`: Stable semantic identity of an existing CAD layer.
    ///
    /// # Returns
    /// Durable source candidate for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns an error for stale, missing, cross-drawing, unsupported, or invalid targets.
    pub fn propose_dxf_layer_change(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        entity: EntityId,
        destination_layer: EntityId,
    ) -> Result<ProjectCandidate, ProjectError> {
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(dxf_candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, entity)
            .map_err(crate::errors::storage_error)?
            .filter(|owned| owned.kind == "cad.entity")
            .ok_or_else(|| {
                dxf_candidate_error("cad_entity_not_found", "target is not an owned CAD entity")
            })?;
        let layer_ownership = self
            .store
            .project_entity(project, destination_layer)
            .map_err(crate::errors::storage_error)?
            .filter(|owned| {
                owned.kind == "cad.layer" && owned.relative_path == ownership.relative_path
            })
            .ok_or_else(|| {
                dxf_candidate_error(
                    "cad_layer_not_found",
                    "destination layer is not owned by the target drawing",
                )
            })?;
        let layer = self.resolve_dxf_layer(base_snapshot, destination_layer)?;
        let revision_id = self
            .store
            .resolve(base_snapshot, entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                dxf_candidate_error("cad_entity_not_in_snapshot", "target is absent from base")
            })?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                dxf_candidate_error(
                    "cad_entity_revision_missing",
                    "CAD entity revision is unavailable",
                )
            })?;
        let semantic: DrawingEntity = serde_json::from_slice(revision.payload())
            .map_err(|_| dxf_candidate_error("cad_entity_invalid", "entity payload is invalid"))?;
        let handle = self.dxf_external_handle(
            project,
            base_snapshot,
            &layer_ownership.relative_path,
            entity,
        )?;
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| {
                file.relative_path == layer_ownership.relative_path && file.adapter == "dxf"
            })
            .ok_or_else(|| dxf_candidate_error("dxf_source_not_found", "CAD source is not DXF"))?;
        self.require_editable_source(project, &source.relative_path, source.digest)?;
        let updated =
            scriberig_domain_dxf::change_entity_layer(&source.bytes, &handle, &layer.name)
                .map_err(|error| {
                    dxf_candidate_entity_error(
                        error.code,
                        &error.message,
                        &source.relative_path,
                        entity,
                    )
                })?;
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "cad.change_layer",
            entity,
        )?;
        self.persist_cad_semantic_change(
            &mut candidate,
            entity,
            "entity.layer",
            serde_json::to_value(semantic.layer)
                .map_err(|_| dxf_candidate_error("cad_diff_invalid", "prior layer is invalid"))?,
            serde_json::to_value(destination_layer).map_err(|_| {
                dxf_candidate_error("cad_diff_invalid", "proposed layer is invalid")
            })?,
        )?;
        Ok(candidate)
    }

    /// Resolves and decodes one exact-snapshot CAD layer revision.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot containing the layer.
    /// - `entity`: Stable semantic layer identity.
    ///
    /// # Returns
    /// The typed layer value from the retained revision.
    ///
    /// # Errors
    /// Returns an error when the layer or its revision payload is unavailable or invalid.
    fn resolve_dxf_layer(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Layer, ProjectError> {
        let revision = self
            .store
            .resolve(snapshot, entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                dxf_candidate_error("cad_layer_not_in_snapshot", "destination layer is absent")
            })?;
        let record = self
            .store
            .get(revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                dxf_candidate_error(
                    "cad_layer_revision_missing",
                    "layer revision is unavailable",
                )
            })?;
        serde_json::from_slice(record.payload())
            .map_err(|_| dxf_candidate_error("cad_layer_invalid", "layer payload is invalid"))
    }

    /// Proposes a text replacement for one imported DXF text entity.
    ///
    /// The operation edits only the authored value while retaining the exact
    /// target identity, position, height, layer, style, and neighboring entities.
    ///
    /// # Parameters
    /// - `project`: Project owning the DXF source.
    /// - `base_snapshot`: Exact active snapshot used to resolve the text entity.
    /// - `entity`: Stable semantic identity of the text entity.
    /// - `replacement`: New authored text without control characters.
    ///
    /// # Returns
    /// Durable source candidate for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns an error for stale, missing, non-text, unsupported, or invalid targets.
    pub fn propose_dxf_text_update(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        entity: EntityId,
        replacement: &str,
    ) -> Result<ProjectCandidate, ProjectError> {
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(dxf_candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, entity)
            .map_err(crate::errors::storage_error)?
            .filter(|owned| owned.kind == "cad.entity")
            .ok_or_else(|| {
                dxf_candidate_error("cad_entity_not_found", "target is not an owned CAD entity")
            })?;
        let revision_id = self
            .store
            .resolve(base_snapshot, entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                dxf_candidate_error(
                    "cad_entity_not_in_snapshot",
                    "CAD entity is absent from base",
                )
            })?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                dxf_candidate_error(
                    "cad_entity_revision_missing",
                    "CAD entity revision is unavailable",
                )
            })?;
        let semantic: DrawingEntity = serde_json::from_slice(revision.payload()).map_err(|_| {
            dxf_candidate_error("cad_entity_invalid", "CAD entity payload is invalid")
        })?;
        if !matches!(&semantic.geometry, CadEntity::Text { .. }) {
            return Err(dxf_candidate_error(
                "dxf_target_not_text",
                "target geometry is not a text entity",
            ));
        }
        let handle =
            self.dxf_external_handle(project, base_snapshot, &ownership.relative_path, entity)?;
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == ownership.relative_path && file.adapter == "dxf")
            .ok_or_else(|| {
                dxf_candidate_error("dxf_source_not_found", "CAD entity source is not DXF")
            })?;
        self.require_editable_source(project, &source.relative_path, source.digest)?;
        let updated = scriberig_domain_dxf::replace_text(&source.bytes, &handle, replacement)
            .map_err(|error| {
                dxf_candidate_entity_error(
                    error.code,
                    &error.message,
                    &source.relative_path,
                    entity,
                )
            })?;
        let before = match &semantic.geometry {
            CadEntity::Text { value, .. } => value.clone(),
            _ => {
                return Err(dxf_candidate_error(
                    "dxf_target_not_text",
                    "target geometry is not a text entity",
                ));
            }
        };
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "cad.update_text",
            entity,
        )?;
        self.persist_cad_semantic_change(
            &mut candidate,
            entity,
            "entity.text",
            serde_json::json!(before),
            serde_json::json!(replacement),
        )?;
        Ok(candidate)
    }
}
