//! Persists typed semantic review values on exact-base DXF candidates.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{ProjectCandidate, ProjectError, ProjectService};

impl ProjectService {
    /// Resolves one source handle from drawing-level provider provenance.
    ///
    /// # Parameters
    /// - `project`: Project owning the DXF source.
    /// - `snapshot`: Exact snapshot containing the drawing summary.
    /// - `source_path`: Project-relative path of the DXF source.
    /// - `entity`: Semantic entity whose external handle is required for rendering.
    ///
    /// # Returns
    /// The DXF handle recorded separately from the semantic entity payload.
    ///
    /// # Errors
    /// Returns an error when drawing provenance is missing, malformed, or ambiguous.
    pub(crate) fn dxf_external_handle(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        source_path: &str,
        entity: EntityId,
    ) -> Result<String, ProjectError> {
        let drawing = self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_debug_error)?
            .into_iter()
            .find(|record| record.relative_path == source_path && record.kind == "cad.drawing")
            .ok_or_else(|| provenance_error("DXF drawing provenance is missing"))?;
        let revision_id = self
            .store
            .resolve(snapshot, drawing.entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| provenance_error("DXF drawing provenance is absent from snapshot"))?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| provenance_error("DXF drawing provenance revision is missing"))?;
        let payload: serde_json::Value = serde_json::from_slice(revision.payload())
            .map_err(|_| provenance_error("DXF drawing provenance payload is invalid"))?;
        let identifiers = payload
            .get("entity_identifiers")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| provenance_error("DXF entity provenance list is missing"))?;
        let matches = identifiers
            .iter()
            .filter_map(|value| {
                serde_json::from_value::<scriberig_domain_cad::CadProviderEntityIdentifier>(
                    value.clone(),
                )
                .ok()
            })
            .filter(|identifier| identifier.entity == entity)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [identifier] => Ok(identifier.value.clone()),
            [] => Err(provenance_error("DXF entity handle provenance is missing")),
            _ => Err(provenance_error(
                "DXF entity handle provenance is ambiguous",
            )),
        }
    }

    /// Persists one bounded semantic change on a source candidate.
    ///
    /// # Parameters
    /// - `candidate`: Exact-base candidate to enrich.
    /// - `target`: Semantic entity changed by the operation.
    /// - `property`: Stable CAD semantic property identifier.
    /// - `before`: Prior typed value from the candidate's base snapshot.
    /// - `after`: Proposed typed value represented by the candidate.
    ///
    /// # Returns
    /// Unit after the semantic change is durable.
    ///
    /// # Errors
    /// Returns an error when the enriched candidate cannot be durably stored.
    pub(crate) fn persist_cad_semantic_change(
        &self,
        candidate: &mut ProjectCandidate,
        target: EntityId,
        property: &str,
        before: serde_json::Value,
        after: serde_json::Value,
    ) -> Result<(), ProjectError> {
        candidate
            .semantic_changes
            .push(crate::SemanticCandidateChange {
                target,
                property: property.to_owned(),
                before,
                after,
            });
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)
    }
}

/// Creates one safe source provenance lookup error.
///
/// # Parameters
/// - `message`: Bounded explanation without provider-controlled text.
///
/// # Returns
/// Structured project error for missing or invalid external identity evidence.
fn provenance_error(message: &str) -> ProjectError {
    ProjectError {
        code: "dxf_provenance_invalid",
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
