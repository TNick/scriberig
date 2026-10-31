//! Implements semantic imports for structured source adapters.

use std::path::Path;

use scriberig_core::{EntityId, ProjectId};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

use super::add_child;

#[path = "adapters/xlsx.rs"]
mod xlsx;
pub(super) use xlsx::import_xlsx;
#[path = "adapters/dxf.rs"]
pub(crate) mod dxf;
pub(super) use dxf::import_dxf;
#[path = "adapters/dwg.rs"]
mod dwg;
pub(super) use dwg::import_dwg;
#[path = "adapters/docx.rs"]
mod docx;
pub(super) use docx::import_docx;
#[path = "adapters/qgis.rs"]
mod qgis;
pub(super) use qgis::import_qgis;
#[path = "adapters/shapefile.rs"]
pub(crate) mod shapefile;
pub(super) use shapefile::import_shapefile;
#[path = "adapters/geopackage.rs"]
pub(crate) mod geopackage;
pub(super) use geopackage::import_geopackage;

#[path = "adapters/binary.rs"]
mod binary;
pub(super) use binary::import_binary;
#[path = "adapters/data.rs"]
mod data;
pub(super) use data::import_data;
#[path = "adapters/text.rs"]
mod text;
pub(super) use text::import_text;

/// Extracts a complete C or C++ syntax tree.
///
/// - `project`: Owning project identity.
/// - `file`: C or C++ source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// Unit after syntax-tree entities are added.
///
/// Returns UTF-8 or C-family parser diagnostics as a retryable project failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn import_c_family(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let text = std::str::from_utf8(&file.bytes).map_err(|error| {
        adapter_diagnostics_error(file, "decode", "decode_failed", &[error.to_string()], true)
    })?;
    let document =
        scriberig_domain_c::parse(Path::new(&file.relative_path), text).map_err(|error| {
            adapter_diagnostics_error(file, "parse", "parse_failed", &error.diagnostics, true)
        })?;
    let document_kind = format!("{}.document", file.adapter);
    let document_payload = serde_json::to_vec(&document).map_err(|error| {
        adapter_diagnostics_error(
            file,
            "encode",
            "encoding_failed",
            &[error.to_string()],
            true,
        )
    })?;
    add_child(
        project,
        file,
        source,
        &document_kind,
        0,
        document_payload,
        revisions,
        changes,
        ownership,
    )?;
    let node_kind = format!("{}.node", file.adapter);
    for (index, node) in document.nodes.iter().enumerate() {
        let payload = serde_json::to_vec(node).map_err(|error| {
            adapter_diagnostics_error(
                file,
                "encode",
                "encoding_failed",
                &[error.to_string()],
                true,
            )
        })?;
        add_child(
            project, file, source, &node_kind, index, payload, revisions, changes, ownership,
        )?;
    }
    Ok(())
}

/// Extracts semantic HTML nodes while retaining the exact source artifact.
///
/// - `project`: Owning project identity.
/// - `file`: HTML source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// Unit after HTML children are added.
///
/// Returns source parsing or revision errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn import_html(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let document = scriberig_domain_html::parse(&file.bytes).map_err(|error| {
        adapter_diagnostics_error(file, "parse", "parse_failed", &[error.to_string()], true)
    })?;
    let document_payload = document.canonical_bytes().map_err(|error| {
        adapter_diagnostics_error(
            file,
            "encode",
            "encoding_failed",
            &[error.to_string()],
            true,
        )
    })?;
    add_child(
        project,
        file,
        source,
        "html.document",
        0,
        document_payload,
        revisions,
        changes,
        ownership,
    )?;
    for (index, node) in document.nodes.iter().enumerate() {
        let payload = node.canonical_bytes().map_err(|error| {
            adapter_diagnostics_error(
                file,
                "encode",
                "encoding_failed",
                &[error.to_string()],
                true,
            )
        })?;
        add_child(
            project,
            file,
            source,
            "html.node",
            index,
            payload,
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}

/// Extracts a complete JavaScript or TypeScript syntax tree.
///
/// - `project`: Owning project identity.
/// - `file`: ECMAScript source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// Unit after syntax-tree entities are added.
///
/// Returns UTF-8 or parser diagnostics as a retryable project failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn import_ecma(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let text = std::str::from_utf8(&file.bytes).map_err(|error| {
        adapter_diagnostics_error(file, "decode", "decode_failed", &[error.to_string()], true)
    })?;
    let document =
        scriberig_domain_ecma::parse(Path::new(&file.relative_path), text).map_err(|error| {
            adapter_diagnostics_error(
                file,
                "parse",
                "parse_failed",
                &error.diagnostics,
                !error.fatal,
            )
        })?;
    let document_payload = serde_json::to_vec(&document).map_err(|error| {
        adapter_diagnostics_error(
            file,
            "encode",
            "encoding_failed",
            &[error.to_string()],
            true,
        )
    })?;
    let document_kind = format!("{}.document", file.adapter);
    add_child(
        project,
        file,
        source,
        &document_kind,
        0,
        document_payload,
        revisions,
        changes,
        ownership,
    )?;
    let node_kind = format!("{}.node", file.adapter);
    for (index, node) in document.nodes.iter().enumerate() {
        let payload = serde_json::to_vec(node).map_err(|error| {
            adapter_diagnostics_error(
                file,
                "encode",
                "encoding_failed",
                &[error.to_string()],
                true,
            )
        })?;
        add_child(
            project, file, source, &node_kind, index, payload, revisions, changes, ownership,
        )?;
    }
    Ok(())
}

/// Extracts a complete Python syntax tree.
///
/// - `project`: Owning project identity.
/// - `file`: Python source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// Unit after syntax-tree entities are added.
///
/// Returns UTF-8 or Python parser diagnostics as a retryable project failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn import_python(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let text = std::str::from_utf8(&file.bytes).map_err(|error| {
        adapter_diagnostics_error(file, "decode", "decode_failed", &[error.to_string()], true)
    })?;
    let document =
        scriberig_domain_python::parse(Path::new(&file.relative_path), text).map_err(|error| {
            adapter_diagnostics_error(file, "parse", "parse_failed", &error.diagnostics, true)
        })?;
    let document_payload = serde_json::to_vec(&document).map_err(|error| {
        adapter_diagnostics_error(
            file,
            "encode",
            "encoding_failed",
            &[error.to_string()],
            true,
        )
    })?;
    add_child(
        project,
        file,
        source,
        "python.document",
        0,
        document_payload,
        revisions,
        changes,
        ownership,
    )?;
    for (index, node) in document.nodes.iter().enumerate() {
        let payload = serde_json::to_vec(node).map_err(|error| {
            adapter_diagnostics_error(
                file,
                "encode",
                "encoding_failed",
                &[error.to_string()],
                true,
            )
        })?;
        add_child(
            project,
            file,
            source,
            "python.node",
            index,
            payload,
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}
