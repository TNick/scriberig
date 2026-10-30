//! Imports `GeoPackage` semantics and features through bounded source pages.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_geo::{GeoDataset, GeoFieldDefinition, GeoPackageReader, semantic_field_id};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};
use serde::Serialize;

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

/// Imports dataset, layer, field, and canonical feature revisions.
///
/// # Parameters
/// - `project`: Owning Scriberig project identity.
/// - `file`: Exact persisted `GeoPackage` source bytes.
/// - `source`: Stable source-artifact identity.
/// - `revisions`: Destination immutable semantic revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project entity ownership records.
///
/// Unit after dataset and layer summaries have been appended.
///
/// Returns a structured package parsing, encoding, or revision error.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
pub(crate) fn import_geopackage(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let dataset_id = dataset_semantic_id(source);
    let reader = GeoPackageReader::open(&file.bytes, dataset_id).map_err(|error| {
        adapter_diagnostics_error(
            file,
            "parse",
            "geopackage_parse_failed",
            &[format!("{error:?}")],
            true,
        )
    })?;
    let dataset_payload = encode(
        file,
        &DatasetSummary {
            dataset: reader.dataset(),
            source_digest: file.digest,
            provider: "rusqlite-0.40.2-staged-geopackage-writer-v1",
            layers: reader.dataset().layers.clone(),
        },
    )?;
    crate::support::add_entity_with_id(
        project,
        file,
        dataset_id,
        "geo.dataset",
        dataset_payload,
        revisions,
        changes,
        ownership,
    )?;
    for layer in reader.layers() {
        crate::support::add_entity_with_id(
            project,
            file,
            layer.layer.id,
            "geo.layer",
            encode(file, layer)?,
            revisions,
            changes,
            ownership,
        )?;
        for definition in &layer.layer.fields {
            let field = GeoFieldDefinition {
                id: semantic_field_id(layer.layer.id, &definition.name),
                layer: layer.layer.id,
                definition: definition.clone(),
            };
            crate::support::add_named_entity_with_id(
                project,
                file,
                field.id,
                "geo.field",
                &field.definition.name,
                encode(file, &field)?,
                revisions,
                changes,
                ownership,
            )?;
        }
        import_layer_features(
            &reader,
            layer.layer.id,
            &mut GeoImportTarget {
                project,
                file,
                revisions,
                changes,
                ownership,
            },
        )?;
    }
    Ok(())
}

/// Imports one layer's semantic features through bounded source pages.
///
/// # Parameters
/// - `reader`: Exact source reader parsed from the persisted package.
/// - `layer`: Stable semantic layer identity.
/// - `target`: Atomic snapshot import outputs.
///
/// # Returns
/// Unit after canonical feature revisions are appended.
///
/// # Errors
/// Returns provider, payload encoding, or revision construction errors.
fn import_layer_features(
    reader: &GeoPackageReader,
    layer: EntityId,
    target: &mut GeoImportTarget<'_>,
) -> Result<(), ProjectError> {
    let mut offset = 0_usize;
    loop {
        let page = reader.feature_page(layer, offset, 1000).map_err(|error| {
            adapter_diagnostics_error(
                target.file,
                "feature_import",
                "geopackage_feature_import_failed",
                &[format!("{error:?}")],
                true,
            )
        })?;
        let count = page.features.len();
        if count == 0 && page.has_more {
            return Err(adapter_diagnostics_error(
                target.file,
                "feature_import",
                "geopackage_feature_page_empty",
                &["provider reported continuation without returning features".to_owned()],
                true,
            ));
        }
        for feature in page.features {
            let name = feature
                .native_id
                .clone()
                .unwrap_or_else(|| feature.id.to_string());
            crate::support::add_named_entity_with_id(
                target.project,
                target.file,
                feature.id,
                "geo.feature",
                &name,
                encode(target.file, &feature)?,
                target.revisions,
                target.changes,
                target.ownership,
            )?;
        }
        if !page.has_more {
            return Ok(());
        }
        offset = offset.saturating_add(count);
    }
}

/// Collects the shared destinations for one package import transaction.
struct GeoImportTarget<'a> {
    /// Owning project identity.
    project: ProjectId,
    /// Persisted exact source record.
    file: &'a ProjectFileRecord,
    /// Immutable revisions staged for the atomic snapshot.
    revisions: &'a mut Vec<Revision>,
    /// Snapshot changes staged for the atomic snapshot.
    changes: &'a mut Vec<SnapshotChange>,
    /// Project ownership rows staged for the atomic snapshot.
    ownership: &'a mut Vec<ProjectEntityRecord>,
}

/// Summarizes dataset identity, layer references, source evidence, and provider.
#[derive(Serialize)]
struct DatasetSummary<'a> {
    /// Provider-neutral logical dataset semantics.
    dataset: &'a GeoDataset,
    /// Exact outer `GeoPackage` byte digest also retained in project evidence.
    source_digest: [u8; 32],
    /// Read-only SQLite adapter and dependency version.
    provider: &'static str,
    /// Ordered semantic layer identities without materializing feature rows.
    layers: Vec<EntityId>,
}

/// Encodes one import summary with file-specific safe diagnostics.
///
/// - `file`: Persisted source context.
/// - `value`: Semantic value to encode.
///
/// Encoded summary bytes.
///
/// Returns an error if serialization fails.
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

/// Derives a stable `GeoPackage` dataset identity from its source artifact.
///
/// # Parameters
/// - `source`: Exact source artifact semantic identity.
///
/// Deterministic Scriberig dataset identity shared by import and reference resolution.
/// # Returns
/// The result of the requested operation.
pub(crate) fn dataset_semantic_id(source: EntityId) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-project-geopackage-v1\0");
    hasher.update(source.as_bytes());
    hasher.update(b"geo/dataset");
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}
