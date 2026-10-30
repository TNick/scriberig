//! Reads bounded snapshot-bound CAD drawing summaries and entity pages.

use std::collections::HashMap;

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_cad::{DrawingEntity, Layer};
use scriberig_store::{RevisionStore, SnapshotStore};
use serde::{Deserialize, Serialize};

use crate::{ProjectError, ProjectService};

/// Maximum CAD entities returned by one project read request.
pub const MAX_CAD_ENTITY_PAGE: usize = 500;

/// Describes bounded drawing metadata and its independently addressable layers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CadDrawingView {
    /// Project owning the drawing.
    pub project: ProjectId,
    /// Exact immutable snapshot used for every record.
    pub snapshot: SnapshotId,
    /// Stable semantic drawing identity.
    pub drawing: EntityId,
    /// Authored or adapter-reported drawing units.
    pub units: String,
    /// Optional coordinate reference label.
    pub coordinate_reference: Option<String>,
    /// Stable semantic layer summaries in drawing order.
    pub layers: Vec<Layer>,
    /// Number of supported imported drawing entities.
    pub entity_count: usize,
    /// Whether the source fully mapped to the supported semantic subset.
    pub complete: bool,
    /// Safe limitations reported during import.
    pub diagnostics: Vec<String>,
}

/// Returns one bounded page of typed CAD entity semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CadEntityPage {
    /// Project owning the drawing.
    pub project: ProjectId,
    /// Exact immutable snapshot used for every entity.
    pub snapshot: SnapshotId,
    /// Stable semantic drawing identity.
    pub drawing: EntityId,
    /// Bounded semantic drawing entities.
    pub entities: Vec<DrawingEntity>,
    /// Total imported entity count for this drawing.
    pub total: usize,
    /// Whether additional entities remain after this page.
    pub has_more: bool,
}

impl ProjectService {
    /// Reads a drawing summary and its layers from an exact active snapshot.
    ///
    /// # Parameters
    /// - `project`: Project owning the drawing.
    /// - `snapshot`: Exact active snapshot to inspect.
    /// - `drawing`: Stable `cad.drawing` identity.
    ///
    /// # Returns
    /// Snapshot-bound drawing summary with typed layers and fidelity diagnostics.
    ///
    /// # Errors
    /// Returns an ownership, stale-snapshot, missing-revision, or decode error.
    pub fn cad_drawing(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        drawing: EntityId,
    ) -> Result<CadDrawingView, ProjectError> {
        self.validate_snapshot(project, snapshot)?;
        let ownership = self.cad_ownership(project)?;
        if ownership.get(&drawing).map(String::as_str) != Some("cad.drawing") {
            return Err(cad_error(
                "cad_drawing_not_found",
                "target is not an owned CAD drawing",
            ));
        }
        let record = self.cad_record(snapshot, drawing)?;
        let value: serde_json::Value = serde_json::from_slice(record.payload())
            .map_err(|_| cad_error("cad_drawing_invalid", "drawing summary is invalid"))?;
        let layers = id_list(&value, "layers")?
            .into_iter()
            .map(|id| {
                self.cad_record(snapshot, id).and_then(|revision| {
                    serde_json::from_slice::<Layer>(revision.payload())
                        .map_err(|_| cad_error("cad_layer_invalid", "layer payload is invalid"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CadDrawingView {
            project,
            snapshot,
            drawing,
            units: string_field(&value, "units")?,
            coordinate_reference: value
                .get("coordinate_reference")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            layers,
            entity_count: id_list(&value, "entities")?.len(),
            complete: value
                .get("complete")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            diagnostics: value
                .get("diagnostics")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    /// Reads a bounded ordered page of CAD entities at an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Project owning the drawing.
    /// - `snapshot`: Exact active snapshot to inspect.
    /// - `drawing`: Stable `cad.drawing` identity.
    /// - `offset`: Zero-based offset in source entity order.
    /// - `limit`: Requested page size between one and five hundred.
    ///
    /// # Returns
    /// Typed entity page with total and continuation metadata.
    ///
    /// # Errors
    /// Returns a request, ownership, stale-snapshot, missing-revision, or decode error.
    pub fn cad_entities(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        drawing: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<CadEntityPage, ProjectError> {
        if !(1..=MAX_CAD_ENTITY_PAGE).contains(&limit) {
            return Err(cad_error(
                "cad_page_limit_invalid",
                "CAD page limit must be between one and five hundred",
            ));
        }
        let summary = self.cad_drawing(project, snapshot, drawing)?;
        let ids = id_list(&self.cad_summary(snapshot, drawing)?, "entities")?;
        if offset > ids.len() {
            return Err(cad_error(
                "cad_page_offset_invalid",
                "CAD page offset exceeds entity count",
            ));
        }
        let entities = ids
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|id| {
                self.cad_record(snapshot, id).and_then(|revision| {
                    serde_json::from_slice::<DrawingEntity>(revision.payload())
                        .map_err(|_| cad_error("cad_entity_invalid", "entity payload is invalid"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let total = summary.entity_count;
        let has_more = offset + entities.len() < total;
        Ok(CadEntityPage {
            project,
            snapshot,
            drawing,
            entities,
            total,
            has_more,
        })
    }

    /// Verifies the project and exact active snapshot for a CAD read.
    /// # Parameters
    /// - `project`: Input used by this operation.
    /// - `snapshot`: Input used by this operation.
    /// # Returns
    /// The result of the requested operation.
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    fn validate_snapshot(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
    ) -> Result<(), ProjectError> {
        if self.workspace(project)?.snapshot != snapshot {
            return Err(cad_error(
                "stale_cad_snapshot",
                "CAD reads require the exact active snapshot",
            ));
        }
        Ok(())
    }

    /// Loads project entity ownership kinds for bounded CAD validation.
    /// # Parameters
    /// - `project`: Input used by this operation.
    /// # Returns
    /// The result of the requested operation.
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    fn cad_ownership(&self, project: ProjectId) -> Result<HashMap<EntityId, String>, ProjectError> {
        Ok(self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_error)?
            .into_iter()
            .map(|owned| (owned.entity, owned.kind))
            .collect())
    }

    /// Resolves one CAD revision from an exact immutable snapshot.
    /// # Parameters
    /// - `snapshot`: Input used by this operation.
    /// - `entity`: Input used by this operation.
    /// # Returns
    /// The result of the requested operation.
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    fn cad_record(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<scriberig_store::Revision, ProjectError> {
        let revision = self
            .store
            .resolve(snapshot, entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                cad_error(
                    "cad_revision_not_found",
                    "CAD entity is absent from the exact snapshot",
                )
            })?;
        self.store
            .get(revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                cad_error(
                    "cad_revision_missing",
                    "CAD revision payload is unavailable",
                )
            })
    }

    /// Resolves and decodes the drawing's compact child identity summary.
    /// # Parameters
    /// - `snapshot`: Input used by this operation.
    /// - `drawing`: Input used by this operation.
    /// # Returns
    /// The result of the requested operation.
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    fn cad_summary(
        &self,
        snapshot: SnapshotId,
        drawing: EntityId,
    ) -> Result<serde_json::Value, ProjectError> {
        let revision = self.cad_record(snapshot, drawing)?;
        serde_json::from_slice(revision.payload())
            .map_err(|_| cad_error("cad_drawing_invalid", "drawing summary is invalid"))
    }
}

/// Reads a serialized array of stable semantic entity identities.
///
/// # Parameters
/// - `value`: Drawing summary object.
/// - `field`: Required child identity field.
///
/// Ordered entity identifiers.
///
/// Returns a semantic decoding error when the field is missing or malformed.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
fn id_list(value: &serde_json::Value, field: &str) -> Result<Vec<EntityId>, ProjectError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            cad_error(
                "cad_drawing_invalid",
                "drawing child identities are invalid",
            )
        })?
        .iter()
        .map(|value| {
            serde_json::from_value::<EntityId>(value.clone())
                .map_err(|_| cad_error("cad_drawing_invalid", "child identity is invalid"))
        })
        .collect()
}

/// Reads one string property from a CAD drawing summary.
///
/// # Parameters
/// - `value`: Serialized drawing summary.
/// - `field`: Required string property.
///
/// Authored field value.
///
/// Returns a semantic decoding error when the property is missing.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
fn string_field(value: &serde_json::Value, field: &str) -> Result<String, ProjectError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| cad_error("cad_drawing_invalid", "drawing string property is invalid"))
}

/// Creates a safe project CAD read error.
///
/// # Parameters
/// - `code`: Stable error category.
/// - `message`: Safe diagnostic without source bytes.
///
/// Structured project read error.
/// # Returns
/// The result of the requested operation.
fn cad_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
