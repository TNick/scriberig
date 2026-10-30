//! Imports provider-supported DWG semantics into the shared CAD schema.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_cad::{CadExternalProvider, CadFormat, CadProviderEntityIdentifier};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};
use serde::Serialize;

use crate::errors::adapter_diagnostics_error;
use crate::{ProjectError, import_service::PreviousCadDrawing};

#[path = "dwg/normalize.rs"]
mod normalize;

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "This adapter stages one drawing across shared immutable revision, change, and ownership buffers; the field groups are consumed together by the import pipeline."
)]
/// Imports provider-converted DWG semantics while retaining exact source bytes.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted exact DWG source evidence.
/// - `source`: Exact source artifact entity.
/// - `provider`: Host-injected provider selected for DWG.
/// - `previous`: Prior provider-derived drawing used for identity reconciliation.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Returns
/// Unit after shared CAD drawing, layer, and entity records are appended.
///
/// # Errors
/// Returns a structured safe provider or encoding diagnostic.
pub(crate) fn import_dwg(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    provider: &dyn CadExternalProvider,
    previous: Option<&PreviousCadDrawing>,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let imported = provider
        .import(&CadFormat::Dwg, &file.bytes)
        .map_err(|error| {
            adapter_diagnostics_error(
                file,
                "provider-import",
                "dwg_provider_import_failed",
                &[format!("provider import failed: {:?}", error.kind)],
                false,
            )
        })?;
    let descriptor = provider.descriptor();
    let expected_capability = descriptor
        .formats
        .iter()
        .find(|capability| capability.format == CadFormat::Dwg);
    let import_identity_matches = imported.provider().id == descriptor.id
        && imported.provider().version == descriptor.version;
    if !import_identity_matches
        || expected_capability.is_none_or(|capability| {
            capability
                .validate_read_support(imported.drawing())
                .is_err()
        })
    {
        return Err(adapter_diagnostics_error(
            file,
            "provider-import",
            "dwg_provider_import_invalid",
            &["provider import exceeded its advertised identity or read capability".to_owned()],
            false,
        ));
    }
    let same_provider = previous.is_some_and(|previous| {
        previous.provider_id.as_deref() == Some(imported.provider().id.as_str())
            && previous.provider_version.as_deref() == Some(imported.provider().version.as_str())
    });
    let (drawing, entity_identifiers) = normalize::provider_drawing(
        source,
        &imported,
        previous.map_or(&[], |previous| previous.entities.as_slice()),
        same_provider,
    )
    .map_err(|error| {
        adapter_diagnostics_error(
            file,
            "provider-import",
            "dwg_provider_import_invalid",
            &[format!("provider import is invalid: {:?}", error.kind)],
            false,
        )
    })?;
    let summary = DrawingSummary {
        id: drawing.id,
        source_artifact: source,
        units: &drawing.units,
        coordinate_reference: &drawing.coordinate_reference,
        layers: drawing.layers.iter().map(|layer| layer.id).collect(),
        entities: drawing.entities.iter().map(|entity| entity.id).collect(),
        blocks: drawing.blocks.iter().map(|block| block.id).collect(),
        layouts: drawing.layouts.iter().map(|layout| layout.id).collect(),
        external_references: drawing
            .external_references
            .iter()
            .map(|reference| reference.id)
            .collect(),
        complete: imported.unmapped_object_count() == 0,
        diagnostics: if imported.unmapped_object_count() == 0 {
            Vec::new()
        } else {
            vec![format!(
                "{} source objects are not represented in shared CAD semantics; DWG remains read-only",
                imported.unmapped_object_count()
            )]
        },
        provider_id: &imported.provider().id,
        provider_version: &imported.provider().version,
        entity_identifiers: &entity_identifiers,
    };
    add_dwg_entity(
        project,
        file,
        drawing.id,
        "cad.drawing",
        &summary,
        revisions,
        changes,
        ownership,
    )?;
    for layer in &drawing.layers {
        add_dwg_entity(
            project,
            file,
            layer.id,
            "cad.layer",
            layer,
            revisions,
            changes,
            ownership,
        )?;
    }
    for entity in &drawing.entities {
        add_dwg_entity(
            project,
            file,
            entity.id,
            "cad.entity",
            entity,
            revisions,
            changes,
            ownership,
        )?;
    }
    for block in &drawing.blocks {
        add_dwg_entity(
            project,
            file,
            block.id,
            "cad.block_definition",
            block,
            revisions,
            changes,
            ownership,
        )?;
    }
    for layout in &drawing.layouts {
        add_dwg_entity(
            project,
            file,
            layout.id,
            "cad.layout",
            layout,
            revisions,
            changes,
            ownership,
        )?;
        for viewport in &layout.viewports {
            add_dwg_entity(
                project,
                file,
                viewport.id,
                "cad.viewport",
                viewport,
                revisions,
                changes,
                ownership,
            )?;
        }
    }
    for reference in &drawing.external_references {
        add_dwg_entity(
            project,
            file,
            reference.id,
            "cad.external_reference",
            reference,
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}

/// Stores drawing-level metadata without duplicating child payloads.
#[derive(Serialize)]
struct DrawingSummary<'a> {
    /// Stable drawing identity.
    id: EntityId,
    /// Exact project source artifact that produced this imported drawing.
    source_artifact: EntityId,
    /// Authored unit label.
    units: &'a str,
    /// Coordinate reference identifier when known.
    coordinate_reference: &'a Option<String>,
    /// Ordered semantic layer identities.
    layers: Vec<EntityId>,
    /// Ordered semantic entity identities.
    entities: Vec<EntityId>,
    /// Ordered block-definition semantic identities.
    blocks: Vec<EntityId>,
    /// Ordered layout semantic identities.
    layouts: Vec<EntityId>,
    /// Ordered external-reference semantic identities.
    external_references: Vec<EntityId>,
    /// Whether provider import represented its declared semantic drawing.
    complete: bool,
    /// Safe read diagnostics from the provider adapter.
    diagnostics: Vec<String>,
    /// Provider identity associated with external native identifiers.
    provider_id: &'a str,
    /// Provider version that produced the imported semantics.
    provider_version: &'a str,
    /// Native handles keyed by semantic entity identity.
    entity_identifiers: &'a [CadProviderEntityIdentifier],
}

/// Encodes and records one provider-imported CAD semantic entity.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Exact source record used for diagnostics.
/// - `entity`: Stable entity identity.
/// - `kind`: Shared CAD entity kind.
/// - `value`: Provider-neutral semantic payload.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Returns
/// Unit after the semantic revision and ownership record are appended.
///
/// # Errors
/// Returns a structured error when serialization or revision construction fails.
#[allow(
    clippy::too_many_arguments,
    reason = "One typed entity is appended atomically to the shared revision, change, and ownership buffers."
)]
fn add_dwg_entity<T: Serialize>(
    project: ProjectId,
    file: &ProjectFileRecord,
    entity: EntityId,
    kind: &str,
    value: &T,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let payload = serde_json::to_vec(value).map_err(|error| {
        adapter_diagnostics_error(
            file,
            "provider-encode",
            "encoding_failed",
            &[error.to_string()],
            true,
        )
    })?;
    crate::support::add_entity_with_id(
        project, file, entity, kind, payload, revisions, changes, ownership,
    )
}
