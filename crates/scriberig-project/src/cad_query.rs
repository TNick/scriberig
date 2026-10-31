//! Queries CAD drawing entities by semantic layer at exact snapshots.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_cad::{CadEntity, DrawingEntity};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{CadEntityPage, MAX_CAD_ENTITY_PAGE, ProjectError, ProjectService};

impl ProjectService {
    /// Reads one bounded page of entities matching a stable CAD geometry kind.
    ///
    /// Kind names are format-independent lowercase identifiers such as
    /// `line`, `polyline`, `circle`, `arc`, `text`, `dimension`, and
    /// `block_reference`.
    ///
    /// # Parameters
    /// - `project`: Project owning the drawing.
    /// - `snapshot`: Exact active snapshot to inspect.
    /// - `drawing`: Stable semantic drawing identity.
    /// - `kind`: Format-independent CAD geometry kind.
    /// - `offset`: Zero-based offset among matching entities.
    /// - `limit`: Requested page size between one and five hundred.
    ///
    /// # Returns
    /// Bounded semantic entity page with total and continuation metadata.
    ///
    /// # Errors
    /// Returns invalid-kind, invalid-page, stale-snapshot, or drawing errors.
    pub fn cad_entities_by_kind(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        drawing: EntityId,
        kind: &str,
        offset: usize,
        limit: usize,
    ) -> Result<CadEntityPage, ProjectError> {
        if !matches!(
            kind,
            "line" | "polyline" | "circle" | "arc" | "text" | "dimension" | "block_reference"
        ) {
            return Err(query_error(
                "cad_entity_kind_invalid",
                "CAD entity kind is not a registered geometry type",
            ));
        }
        self.query_cad_entities(project, snapshot, drawing, offset, limit, |entity| {
            entity_kind(&entity.geometry) == kind
        })
    }

    /// Resolves one native CAD handle to its semantic entity at an exact snapshot.
    ///
    /// The external handle is searched only as source evidence within the
    /// requested drawing. The returned identity is always the Scriberig
    /// semantic entity ID stored in that snapshot.
    ///
    /// # Parameters
    /// - `project`: Project owning the drawing.
    /// - `snapshot`: Exact active snapshot to inspect.
    /// - `drawing`: Stable semantic drawing identity.
    /// - `native_handle`: Non-empty external format handle.
    ///
    /// # Returns
    /// The semantic entity associated with exactly one matching source handle.
    ///
    /// # Errors
    /// Returns invalid-handle, missing-handle, ambiguous-handle, or snapshot
    /// errors when the lookup cannot be resolved uniquely.
    pub fn cad_entity_by_native_handle(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        drawing: EntityId,
        native_handle: &str,
    ) -> Result<DrawingEntity, ProjectError> {
        if native_handle.trim().is_empty() {
            return Err(query_error(
                "cad_native_handle_invalid",
                "native CAD handle is empty",
            ));
        }
        self.store
            .project_entity(project, drawing)
            .map_err(crate::errors::storage_error)?
            .filter(|record| record.kind == "cad.drawing")
            .ok_or_else(|| query_error("cad_drawing_not_found", "drawing is not project-owned"))?;
        let drawing_revision = self
            .store
            .resolve(snapshot, drawing)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                query_error("cad_drawing_not_found", "drawing is absent from snapshot")
            })?;
        let drawing_payload = self
            .store
            .get(drawing_revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| query_error("cad_drawing_not_found", "drawing revision is missing"))?;
        let summary: serde_json::Value = serde_json::from_slice(drawing_payload.payload())
            .map_err(|_| query_error("cad_drawing_invalid", "drawing summary is invalid"))?;
        let identifiers = summary
            .get("entity_identifiers")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                query_error("cad_provenance_missing", "drawing provenance is missing")
            })?;
        let matched = identifiers
            .iter()
            .filter_map(|value| {
                serde_json::from_value::<scriberig_domain_cad::CadProviderEntityIdentifier>(
                    value.clone(),
                )
                .ok()
            })
            .filter(|identifier| identifier.value == native_handle)
            .map(|identifier| identifier.entity)
            .collect::<Vec<_>>();
        let entity = match matched.as_slice() {
            [entity] => *entity,
            [] => {
                return Err(query_error(
                    "cad_native_handle_not_found",
                    "native CAD handle is not present in the requested drawing snapshot",
                ));
            }
            _ => {
                return Err(query_error(
                    "cad_native_handle_ambiguous",
                    "native CAD handle resolves to multiple semantic entities",
                ));
            }
        };
        let revision = self
            .store
            .resolve(snapshot, entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| query_error("cad_entity_not_found", "semantic entity is absent"))?;
        let record = self
            .store
            .get(revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| query_error("cad_entity_not_found", "semantic revision is missing"))?;
        serde_json::from_slice(record.payload())
            .map_err(|_| query_error("cad_entity_invalid", "semantic entity is invalid"))
    }

    /// Reads one bounded page of entities belonging to a drawing layer.
    ///
    /// The filter resolves against the drawing's semantic layer identities and
    /// preserves source order. The scan reads existing bounded entity pages;
    /// it does not materialize every drawing entity in the returned response.
    ///
    /// # Parameters
    /// - `project`: Project owning the drawing.
    /// - `snapshot`: Exact active snapshot to inspect.
    /// - `drawing`: Stable semantic drawing identity.
    /// - `layer`: Stable semantic layer identity belonging to the drawing.
    /// - `offset`: Zero-based offset among matching entities.
    /// - `limit`: Requested page size between one and five hundred.
    ///
    /// # Returns
    /// Bounded semantic entity page with total and continuation metadata.
    ///
    /// # Errors
    /// Returns invalid-page, stale-snapshot, missing-drawing, or missing-layer
    /// errors when the requested query cannot be resolved.
    pub fn cad_entities_by_layer(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        drawing: EntityId,
        layer: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<CadEntityPage, ProjectError> {
        let view = self.cad_drawing(project, snapshot, drawing)?;
        if !view.layers.iter().any(|candidate| candidate.id == layer) {
            return Err(query_error(
                "cad_layer_not_found",
                "layer is not part of the requested CAD drawing",
            ));
        }
        self.query_cad_entities(project, snapshot, drawing, offset, limit, |entity| {
            entity.layer == layer
        })
    }

    /// Scans snapshot entity pages and applies one semantic filter.
    ///
    /// # Parameters
    /// - `project`: Project owning the drawing.
    /// - `snapshot`: Exact active snapshot to inspect.
    /// - `drawing`: Stable semantic drawing identity.
    /// - `offset`: Zero-based offset among matching entities.
    /// - `limit`: Requested page size between one and five hundred.
    /// - `matches_entity`: Predicate selecting entities for the result.
    ///
    /// # Returns
    /// Bounded semantic entity page with total and continuation metadata.
    ///
    /// # Errors
    /// Returns invalid-page, stale-snapshot, missing-drawing, or page errors.
    pub(crate) fn query_cad_entities(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        drawing: EntityId,
        offset: usize,
        limit: usize,
        matches_entity: impl Fn(&DrawingEntity) -> bool,
    ) -> Result<CadEntityPage, ProjectError> {
        if !(1..=MAX_CAD_ENTITY_PAGE).contains(&limit) {
            return Err(query_error(
                "cad_page_limit_invalid",
                "CAD page limit must be between one and five hundred",
            ));
        }
        let view = self.cad_drawing(project, snapshot, drawing)?;
        let mut matches = Vec::with_capacity(limit.saturating_add(1));
        let mut total = 0usize;
        let mut scan_offset = 0usize;
        while scan_offset < view.entity_count {
            let page =
                self.cad_entities(project, snapshot, drawing, scan_offset, MAX_CAD_ENTITY_PAGE)?;
            let page_len = page.entities.len();
            for entity in page.entities {
                if matches_entity(&entity) {
                    if total >= offset && matches.len() <= limit {
                        matches.push(entity);
                    }
                    total = total.saturating_add(1);
                }
            }
            scan_offset += page_len;
        }
        if offset > total {
            return Err(query_error(
                "cad_page_offset_invalid",
                "CAD page offset exceeds the matching entity count",
            ));
        }
        let has_more = matches.len() > limit;
        Ok(CadEntityPage {
            project,
            snapshot,
            drawing,
            entities: matches.into_iter().take(limit).collect(),
            total,
            has_more,
        })
    }
}

/// Returns the stable format-independent kind of a CAD geometry value.
///
/// # Parameters
/// - `geometry`: Typed CAD geometry to classify.
///
/// # Returns
/// Registered lowercase geometry kind identifier.
fn entity_kind(geometry: &CadEntity) -> &'static str {
    match geometry {
        CadEntity::Line { .. } => "line",
        CadEntity::Polyline { .. } => "polyline",
        CadEntity::Circle { .. } => "circle",
        CadEntity::Arc { .. } => "arc",
        CadEntity::Text { .. } => "text",
        CadEntity::Dimension { .. } => "dimension",
        CadEntity::BlockReference(_) => "block_reference",
    }
}

/// Creates a structured CAD query error.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// Project error without source package bytes.
fn query_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
