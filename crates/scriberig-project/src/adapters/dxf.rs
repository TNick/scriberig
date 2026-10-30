//! Imports supported DXF drawing, layer, and entity semantics.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_cad::CadProviderEntityIdentifier;
use scriberig_domain_dxf::DxfPreviousEntity;
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};
use serde::Serialize;

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

/// Imports bounded semantic DXF entities while retaining exact source bytes.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted source DXF.
/// - `source`: Exact source artifact entity.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project entity ownership records.
/// - `previous_entities`: Prior semantic entities with detached DXF IDs.
///
/// Unit after the supported drawing semantics have been appended.
///
/// Returns parser, serialization, or semantic revision diagnostics.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
pub(crate) fn import_dxf(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
    previous_entities: &[DxfPreviousEntity],
) -> Result<(), ProjectError> {
    let imported = scriberig_domain_dxf::import_dxf_reconciled(
        &file.bytes,
        drawing_id(source),
        previous_entities,
    )
    .map_err(|error| {
        adapter_diagnostics_error(file, "parse", "dxf_parse_failed", &[error], true)
    })?;
    let drawing = imported.drawing();
    let drawing_payload = encode(
        file,
        &DrawingSummary {
            id: drawing.id,
            units: &drawing.units,
            coordinate_reference: &drawing.coordinate_reference,
            layers: drawing.layers.iter().map(|layer| layer.id).collect(),
            entities: drawing.entities.iter().map(|entity| entity.id).collect(),
            complete: imported.complete,
            diagnostics: &imported.diagnostics,
            provider_id: &imported.provider_import.provider().id,
            provider_version: &imported.provider_import.provider().version,
            entity_identifiers: imported.provider_import.entity_identifiers(),
        },
    )?;
    crate::support::add_entity_with_id(
        project,
        file,
        drawing.id,
        "cad.drawing",
        drawing_payload,
        revisions,
        changes,
        ownership,
    )?;
    for layer in &drawing.layers {
        crate::support::add_entity_with_id(
            project,
            file,
            layer.id,
            "cad.layer",
            encode(file, layer)?,
            revisions,
            changes,
            ownership,
        )?;
    }
    for entity in &drawing.entities {
        crate::support::add_entity_with_id(
            project,
            file,
            entity.id,
            "cad.entity",
            encode(file, entity)?,
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}

/// Derives the stable semantic drawing identity from its source artifact.
///
/// # Parameters
/// - `source`: Exact project-file entity identity.
///
/// # Returns
/// Stable drawing identity used by DXF imports and candidate validation.
pub(crate) fn drawing_id(source: EntityId) -> EntityId {
    derive_id(source, b"cad.drawing", b"root")
}

/// Stores drawing-level metadata without duplicating all child payloads.
#[derive(Serialize)]
struct DrawingSummary<'a> {
    /// Stable drawing identity.
    id: EntityId,
    /// Authored drawing unit label.
    units: &'a str,
    /// Coordinate reference system identifier when known.
    coordinate_reference: &'a Option<String>,
    /// Ordered layer semantic identities.
    layers: Vec<EntityId>,
    /// Ordered entity semantic identities.
    entities: Vec<EntityId>,
    /// Whether all source entities were represented semantically.
    complete: bool,
    /// Safe read-only reasons for unsupported drawing content.
    diagnostics: &'a [String],
    /// Provider identity associated with external entity identifiers.
    provider_id: &'a str,
    /// Provider version that produced the entity identifiers.
    provider_version: &'a str,
    /// Native handles keyed by semantic entity identity, stored outside entity payloads.
    entity_identifiers: &'a [CadProviderEntityIdentifier],
}

/// Encodes one semantic record with safe source-specific diagnostics.
///
/// - file: Persisted source used for diagnostic context.
/// - value: Semantic value to encode.
///
/// Serialized semantic record.
///
/// Returns a retryable project diagnostic when encoding fails.
fn encode<T: Serialize>(file: &ProjectFileRecord, value: &T) -> Result<Vec<u8>, ProjectError> {
    serde_json::to_vec(value).map_err(|error| {
        adapter_diagnostics_error(
            file,
            "encode",
            "encoding_failed",
            &[error.to_string()],
            true,
        )
    })
}

/// Derives stable DXF semantic identities from source and local keys.
///
/// # Parameters
/// - `parent`: Stable parent semantic identity.
/// - `namespace`: Adapter-owned identity namespace.
/// - `key`: Stable local source key.
///
/// Deterministic semantic entity identity.
/// # Returns
/// The result of the requested operation.
fn derive_id(parent: EntityId, namespace: &[u8], key: &[u8]) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-project-dxf-v1\0");
    hasher.update(parent.as_bytes());
    hasher.update(namespace);
    hasher.update(key);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::import_dxf;
    use dxf::{
        Drawing, Point,
        entities::{Entity, EntityType, ModelPoint},
    };
    use scriberig_core::{EntityId, ProjectId};
    use scriberig_store::SnapshotChange;
    use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

    /// Persists unsupported DXF import diagnostics with the drawing summary.
    #[test]
    fn unsupported_entities_downgrade_persisted_drawing_summary() {
        let mut drawing = Drawing::new();
        drawing.add_entity(Entity::new(EntityType::ModelPoint(ModelPoint::new(
            Point::new(1.0, 2.0, 0.0),
        ))));
        let mut bytes = Vec::new();
        drawing.save(&mut bytes).expect("serialize DXF fixture");
        assert!(!scriberig_domain_dxf::supports_opaque_preserving_edits(
            &bytes
        ));
        let project = ProjectId::fresh();
        let source = EntityId::new();
        let file = ProjectFileRecord {
            project_id: project,
            relative_path: "drawing.dxf".to_owned(),
            entity: source,
            digest: *blake3::hash(&bytes).as_bytes(),
            bytes,
            adapter: "dxf".to_owned(),
            status: "ready".to_owned(),
            grouping_key: None,
        };
        let mut revisions = Vec::new();
        let mut changes = Vec::<SnapshotChange>::new();
        let mut ownership = Vec::<ProjectEntityRecord>::new();
        import_dxf(
            project,
            &file,
            source,
            &mut revisions,
            &mut changes,
            &mut ownership,
            &[],
        )
        .expect("import supported drawing subset");
        let summary = revisions
            .iter()
            .map(|revision| {
                serde_json::from_slice::<serde_json::Value>(revision.payload())
                    .expect("drawing summary JSON")
            })
            .find(|payload| payload.get("complete").is_some())
            .expect("drawing summary");
        assert_eq!(summary["complete"], false);
        let diagnostics = summary["diagnostics"].as_array().expect("diagnostics");
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .as_str()
                .is_some_and(|message| message.contains("unsupported DXF entity"))
        }));
    }
}
