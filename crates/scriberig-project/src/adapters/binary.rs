//! Imports binary project artifacts as metadata documents.

use std::path::Path;

use scriberig_core::{EntityId, ProjectId};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

use super::add_child;

/// Extracts binary metadata while retaining exact source bytes in the parent.
///
/// - `project`: Owning project identity.
/// - `file`: Binary source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
/// - `super`: Parent support module visibility marker.
///
/// Unit after the binary metadata entity is added.
///
/// Returns serialization or revision errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn import_binary(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let document = scriberig_domain_binary::parse(Path::new(&file.relative_path), &file.bytes);
    let payload = serde_json::to_vec(&document).map_err(|error| {
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
        "binary.document",
        0,
        payload,
        revisions,
        changes,
        ownership,
    )
}
