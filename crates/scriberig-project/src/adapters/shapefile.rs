//! Imports compound Shapefile semantics through the shared geospatial domain.

use std::path::{Path, PathBuf};

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_geo::{GeoDataset, GeoFieldDefinition, ShapefileReader, semantic_field_id};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};
use serde::Serialize;

use crate::{ProjectError, errors::adapter_diagnostics_error};

/// Imports a compound Shapefile as canonical shared geo dataset, layer, field, and feature entities.
///
/// # Parameters
/// - `project`: Owning Scriberig project identity.
/// - `file`: Exact persisted `.shp` geometry member.
/// - `manifest`: Exact persisted scan with all compound sidecar evidence.
/// - `source`: Stable source artifact identity.
/// - `revisions`: Destination immutable semantic revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Returns
/// Unit after all geospatial revisions from this shapefile are appended.
///
/// # Errors
/// Returns malformed source, sidecar, encoding, or semantic revision errors.
pub(crate) fn import_shapefile(
    project: ProjectId,
    file: &ProjectFileRecord,
    manifest: &[ProjectFileRecord],
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let stem = Path::new(&file.relative_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| shapefile_error(file, "shapefile_path_invalid", "source stem is invalid"))?;
    let dbf = sidecar(file, manifest, "dbf");
    let prj = sidecar(file, manifest, "prj");
    let reader = ShapefileReader::open(
        &file.bytes,
        dbf.map(|member| member.bytes.as_slice()),
        prj.map(|member| member.bytes.as_slice()),
        dataset_semantic_id(source),
        stem,
    )
    .map_err(|error| {
        adapter_diagnostics_error(
            file,
            "parse",
            "shapefile_parse_failed",
            &[format!("{error:?}")],
            true,
        )
    })?;
    let summary = reader.layer();
    append_shapefile_metadata(project, file, &summary, revisions, changes, ownership)?;
    append_shapefile_features(
        file, &reader, &summary, project, revisions, changes, ownership,
    )
}

/// Appends dataset, layer, and field revisions from one Shapefile summary.
///
/// # Parameters
/// - `project`: Owning Scriberig project.
/// - `file`: Exact `.shp` source record.
/// - `summary`: Provider-neutral dataset, layer, and read-only metadata.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership rows.
///
/// # Returns
/// Unit after metadata revisions are appended.
///
/// # Errors
/// Returns encoding or semantic revision errors.
fn append_shapefile_metadata(
    project: ProjectId,
    file: &ProjectFileRecord,
    summary: &scriberig_domain_geo::ShapefileLayer,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let dataset = serialize(
        file,
        &ShapefileDatasetSummary {
            dataset: summary.dataset.clone(),
            source_digest: file.digest,
            provider: "scriberig-domain-geo-shapefile-2d-readonly-v1",
            layers: summary.dataset.layers.clone(),
            feature_count: summary.feature_count,
            diagnostics: summary.diagnostics.clone(),
        },
    )?;
    crate::support::add_entity_with_id(
        project,
        file,
        summary.dataset.id,
        "geo.dataset",
        dataset,
        revisions,
        changes,
        ownership,
    )?;
    crate::support::add_entity_with_id(
        project,
        file,
        summary.layer.id,
        "geo.layer",
        serialize(file, &summary.layer)?,
        revisions,
        changes,
        ownership,
    )?;
    for field in &summary.layer.fields {
        let definition = GeoFieldDefinition {
            id: semantic_field_id(summary.layer.id, &field.name),
            layer: summary.layer.id,
            definition: field.clone(),
        };
        crate::support::add_named_entity_with_id(
            project,
            file,
            definition.id,
            "geo.field",
            &definition.definition.name,
            serialize(file, &definition)?,
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}

/// Appends canonical feature revisions in bounded source pages.
///
/// # Parameters
/// - `file`: Exact persisted `.shp` source record.
/// - `reader`: Validated bounded Shapefile source adapter.
/// - `summary`: Shared semantic layer identity and source count.
/// - `project`: Owning Scriberig project.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership rows.
///
/// # Returns
/// Unit after all feature revisions are appended.
///
/// # Errors
/// Returns source, encoding, or semantic revision errors.
fn append_shapefile_features(
    file: &ProjectFileRecord,
    reader: &ShapefileReader<'_>,
    summary: &scriberig_domain_geo::ShapefileLayer,
    project: ProjectId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let mut offset = 0_usize;
    loop {
        let page = reader
            .feature_page(summary.layer.id, offset, 1_000)
            .map_err(|error| {
                adapter_diagnostics_error(
                    file,
                    "parse",
                    "shapefile_feature_page_failed",
                    &[format!("{error:?}")],
                    true,
                )
            })?;
        for feature in page.features {
            crate::support::add_entity_with_id(
                project,
                file,
                feature.id,
                "geo.feature",
                serialize(file, &feature)?,
                revisions,
                changes,
                ownership,
            )?;
        }
        if !page.has_more {
            break;
        }
        offset = offset.checked_add(1_000).ok_or_else(|| {
            shapefile_error(file, "shapefile_offset_overflow", "feature offset overflow")
        })?;
    }
    Ok(())
}

/// Opens one compound Shapefile set from an exact persisted scan.
///
/// # Parameters
/// - `shape_file`: Exact `.shp` source record.
/// - `manifest`: Persisted project scan members.
/// - `dataset`: Stable shared dataset identity.
///
/// # Returns
/// Bounded reader over the source and matching optional sidecars.
///
/// # Errors
/// Returns an error when a required source member cannot be parsed.
pub(crate) fn open_shapefile<'a>(
    shape_file: &'a ProjectFileRecord,
    manifest: &'a [ProjectFileRecord],
    dataset: EntityId,
) -> Result<ShapefileReader<'a>, scriberig_domain_geo::ShapefileError> {
    let stem = Path::new(&shape_file.relative_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or(scriberig_domain_geo::ShapefileError::InvalidShape)?;
    ShapefileReader::open(
        &shape_file.bytes,
        sidecar(shape_file, manifest, "dbf").map(|member| member.bytes.as_slice()),
        sidecar(shape_file, manifest, "prj").map(|member| member.bytes.as_slice()),
        dataset,
        stem,
    )
}

/// Derives a stable Shapefile dataset identity from its exact source member.
///
/// # Parameters
/// - `source`: Source artifact entity identity.
///
/// # Returns
/// Deterministic dataset identity for the Shapefile group.
pub(crate) fn dataset_semantic_id(source: EntityId) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-project-shapefile-v1\0");
    hasher.update(source.as_bytes());
    hasher.update(b"geo/dataset");
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Locates one optional same-stem sidecar from the exact project scan.
///
/// # Parameters
/// - `shape_file`: Exact source geometry member.
/// - `manifest`: Persisted project file records.
/// - `extension`: Sidecar suffix without a leading dot.
///
/// # Returns
/// Matching case-insensitive sidecar, when the scan contains it.
fn sidecar<'a>(
    shape_file: &ProjectFileRecord,
    manifest: &'a [ProjectFileRecord],
    extension: &str,
) -> Option<&'a ProjectFileRecord> {
    let mut expected = PathBuf::from(&shape_file.relative_path);
    expected.set_extension(extension);
    let expected = expected.to_string_lossy();
    manifest
        .iter()
        .find(|member| member.relative_path.eq_ignore_ascii_case(&expected))
}

/// Encodes one semantic Shapefile entity with safe source diagnostics on error.
///
/// # Parameters
/// - `file`: Source record for diagnostic attribution.
/// - `value`: Provider-neutral semantic entity.
///
/// # Returns
/// Serialized semantic payload bytes.
///
/// # Errors
/// Returns a structured encoding error without source contents.
fn serialize<T: Serialize>(file: &ProjectFileRecord, value: &T) -> Result<Vec<u8>, ProjectError> {
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

/// Stores provider and fidelity metadata for one imported Shapefile dataset.
#[derive(Serialize)]
struct ShapefileDatasetSummary {
    /// Stable semantic dataset identity and layer membership.
    dataset: GeoDataset,
    /// Digest of the exact `.shp` source member.
    source_digest: [u8; 32],
    /// Versioned reader implementation identity.
    provider: &'static str,
    /// Ordered semantic layer identities.
    layers: Vec<EntityId>,
    /// Count of rows without materializing the source.
    feature_count: u64,
    /// Safe source/provider diagnostics.
    diagnostics: Vec<String>,
}

/// Creates a safe parse error attributed to one persisted source member.
///
/// # Parameters
/// - `file`: Exact `.shp` source record.
/// - `code`: Stable error category.
/// - `message`: Safe explanation without source bytes.
///
/// # Returns
/// Structured project adapter error.
fn shapefile_error(file: &ProjectFileRecord, code: &'static str, message: &str) -> ProjectError {
    adapter_diagnostics_error(file, "parse", code, &[message.to_owned()], true)
}
