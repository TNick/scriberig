//! Imports QGIS project configuration and layer references separately.

use scriberig_core::{EntityId, ProjectId};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};
use std::path::{Component, Path, PathBuf};

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

/// Imports QGIS project metadata and independent layer-reference entities.
///
/// # Parameters
/// - `project`: Owning Scriberig project identity.
/// - `file`: Persisted QGS or QGZ source.
/// - `manifest`: Exact persisted scan for reference resolution.
/// - `source`: Exact source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Returns
/// Unit after QGIS project and reference entities are appended.
///
/// # Errors
/// Returns package parsing, serialization, or semantic revision diagnostics.
pub(crate) fn import_qgis(
    project: ProjectId,
    file: &ProjectFileRecord,
    manifest: &[ProjectFileRecord],
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let imported =
        scriberig_domain_qgis::import_qgis_project(&file.bytes, derive_id(source, "qgis/project"))
            .map_err(|error| {
                adapter_diagnostics_error(file, "parse", "qgis_parse_failed", &[error], true)
            })?;
    let mut qgis_project = imported.project;
    resolve_layer_references(file, manifest, &mut qgis_project);
    let root = serde_json::to_vec(&qgis_project).map_err(|error| {
        adapter_diagnostics_error(
            file,
            "encode",
            "encoding_failed",
            &[error.to_string()],
            true,
        )
    })?;
    crate::support::add_entity_with_id(
        project,
        file,
        qgis_project.id,
        "qgis.project",
        root,
        revisions,
        changes,
        ownership,
    )?;
    for layer in &qgis_project.layers {
        let payload = serde_json::to_vec(layer).map_err(|error| {
            adapter_diagnostics_error(
                file,
                "encode",
                "encoding_failed",
                &[error.to_string()],
                true,
            )
        })?;
        crate::support::add_entity_with_id(
            project,
            file,
            layer.id,
            "qgis.layer_reference",
            payload,
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}

/// Resolves safe project-relative references against the exact persisted manifest.
///
/// # Parameters
/// - `qgis_file`: QGS/QGZ source record whose path anchors relative references.
/// - `manifest`: Exact persisted project scan records.
/// - `qgis_project`: Parsed project mutated with resolved dataset/layer IDs or diagnostics.
fn resolve_layer_references(
    qgis_file: &ProjectFileRecord,
    manifest: &[ProjectFileRecord],
    qgis_project: &mut scriberig_domain_qgis::QgisProject,
) {
    for reference in &mut qgis_project.layers {
        let Some(path) = reference.source.path.as_deref() else {
            reference.resolution_diagnostic = Some(
                "external QGIS data source is not a project-relative supported vector dataset"
                    .into(),
            );
            continue;
        };
        let Some(relative_path) = project_relative_path(&qgis_file.relative_path, path) else {
            reference.resolution_diagnostic =
                Some("QGIS data source escapes the project-relative path boundary".into());
            continue;
        };
        reference.resolved_path = Some(relative_path.clone());
        let Some(source) = manifest.iter().find(|candidate| {
            candidate.relative_path == relative_path
                && matches!(candidate.adapter.as_str(), "geopackage" | "shapefile")
        }) else {
            reference.resolution_diagnostic =
                Some("referenced vector dataset is absent from the persisted project scan".into());
            continue;
        };
        let resolved = match source.adapter.as_str() {
            "geopackage" => resolve_geopackage_reference(source, reference),
            "shapefile" => resolve_shapefile_reference(source, manifest, reference),
            _ => None,
        };
        if let Some((dataset, layer)) = resolved {
            reference.dataset = Some(dataset);
            reference.layer = Some(layer);
            reference.resolution_diagnostic = None;
        } else {
            reference.resolution_diagnostic = Some(
                "referenced vector layer could not be resolved from the captured source".into(),
            );
        }
    }
}

/// Resolves one QGIS reference against a `GeoPackage` source.
///
/// # Parameters
/// - `source`: Persisted `GeoPackage` project member.
/// - `reference`: Authored project layer reference.
///
/// # Returns
/// Shared dataset and layer identities when the source layer matches.
fn resolve_geopackage_reference(
    source: &ProjectFileRecord,
    reference: &mut scriberig_domain_qgis::QgisLayerReference,
) -> Option<(scriberig_core::EntityId, scriberig_core::EntityId)> {
    let reader = scriberig_domain_geo::GeoPackageReader::open(
        &source.bytes,
        super::geopackage::dataset_semantic_id(source.entity),
    )
    .ok()?;
    let layer = reader.layers().iter().find(|candidate| {
        reference.source_layer.as_deref().map_or_else(
            || reference.name == candidate.layer.name,
            |name| name == candidate.table_name,
        )
    })?;
    Some((reader.dataset().id, layer.layer.id))
}

/// Resolves one QGIS reference against a compound Shapefile source.
///
/// # Parameters
/// - `source`: Persisted `.shp` geometry member.
/// - `manifest`: Exact persisted scan containing compound sidecars.
/// - `reference`: Authored project layer reference.
///
/// # Returns
/// Shared dataset and layer identities when the source layer matches.
fn resolve_shapefile_reference(
    source: &ProjectFileRecord,
    manifest: &[ProjectFileRecord],
    reference: &scriberig_domain_qgis::QgisLayerReference,
) -> Option<(scriberig_core::EntityId, scriberig_core::EntityId)> {
    let dataset = super::shapefile::dataset_semantic_id(source.entity);
    let reader = super::shapefile::open_shapefile(source, manifest, dataset).ok()?;
    let layer = reader.layer();
    let matches = reference.source_layer.as_deref().map_or_else(
        || reference.name.eq_ignore_ascii_case(&layer.layer.name),
        |name| name.eq_ignore_ascii_case(&layer.layer.name),
    );
    matches.then_some((layer.dataset.id, layer.layer.id))
}

/// Resolves a relative datasource lexically without allowing project-root escape.
///
/// # Parameters
/// - `project_file`: Project-relative path of the QGIS project.
/// - `source_path`: Authored relative datasource path.
///
/// # Returns
/// Normalized project-relative POSIX path, or `None` for an absolute/escaping path.
fn project_relative_path(project_file: &str, source_path: &str) -> Option<String> {
    let source_path = source_path.replace('\\', "/");
    let project_parent = Path::new(project_file)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let mut normalized = PathBuf::new();
    for component in project_parent.join(source_path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(value) => normalized.push(value),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized.to_string_lossy().replace('\\', "/"))
}

/// Derives a stable project semantic identity from an owning source and key.
///
/// # Parameters
/// - `parent`: Exact source artifact identity.
/// - `key`: Stable QGIS project key.
///
/// # Returns
/// Deterministic project identity.
fn derive_id(parent: EntityId, key: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-project-qgis-v1\0");
    hasher.update(parent.as_bytes());
    hasher.update(key.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}
