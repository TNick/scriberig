//! Implements semantic imports for structured source adapters.

use std::path::Path;

use scriberig_core::{EntityId, ProjectId};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

use super::add_child;

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
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: C or C++ source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// # Returns
/// Unit after syntax-tree entities are added.
///
/// # Errors
/// Returns UTF-8 or C-family parser diagnostics as a retryable project failure.
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
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: HTML source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// # Returns
/// Unit after HTML children are added.
///
/// # Errors
/// Returns source parsing or revision errors.
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
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: ECMAScript source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// # Returns
/// Unit after syntax-tree entities are added.
///
/// # Errors
/// Returns UTF-8 or parser diagnostics as a retryable project failure.
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
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Python source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// # Returns
/// Unit after syntax-tree entities are added.
///
/// # Errors
/// Returns UTF-8 or Python parser diagnostics as a retryable project failure.
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
