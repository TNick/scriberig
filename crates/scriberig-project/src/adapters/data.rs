//! Imports structured JSON, YAML, and TOML project files.

use std::path::Path;

use scriberig_core::{EntityId, ProjectId};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

use super::add_child;

/// Extracts a structured JSON, YAML, or TOML document and value nodes.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Structured UTF-8 file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// # Returns
/// Unit after the structured document and value entities are added.
///
/// # Errors
/// Returns UTF-8, parser, serialization, or revision errors.
pub fn import_data(
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
        scriberig_domain_data::parse(Path::new(&file.relative_path), text).map_err(|error| {
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
