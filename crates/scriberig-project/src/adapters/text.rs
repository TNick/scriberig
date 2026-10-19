//! Imports generic UTF-8 project files as deterministic line documents.

use scriberig_core::{EntityId, ProjectId};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

use super::add_child;

/// Extracts a generic UTF-8 text document and its ordered lines.
///
/// - `project`: Owning project identity.
/// - `file`: UTF-8 text file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// Unit after the text document and line entities are added.
///
/// Returns invalid UTF-8, serialization, or revision errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn import_text(
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
    let document = scriberig_domain_text::parse(text);
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
        "text.document",
        0,
        document_payload,
        revisions,
        changes,
        ownership,
    )?;
    for line in document.lines {
        let payload = serde_json::to_vec(&line).map_err(|error| {
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
            "text.line",
            line.index,
            payload,
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}
