//! Decodes persisted multi-project catalog rows.

use crate::project_files::{ProjectEntityRecord, ProjectFileRecord};
use rusqlite::Row;
use scriberig_core::{EntityId, ProjectId};

/// Decodes one persisted project-file row.
///
/// - `row`: SQLite row containing a project-file record.
/// - `crate`: Internal visibility marker.
///
/// A decoded project-file record.
///
/// Returns an identifier conversion error for malformed stored data.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn file_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectFileRecord> {
    let project: Vec<u8> = row.get(0)?;
    let entity: Vec<u8> = row.get(2)?;
    let digest: Vec<u8> = row.get(3)?;
    let project = fixed_id(project, "project", 0)?.map(ProjectId::from_bytes);
    let entity = fixed_id(entity, "entity", 2)?.map(EntityId::from_bytes);
    let digest = digest.try_into().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Blob,
            Box::new(std::io::Error::other("invalid project file digest")),
        )
    })?;
    Ok(ProjectFileRecord {
        project_id: project.ok_or(rusqlite::Error::InvalidQuery)?,
        relative_path: row.get(1)?,
        entity: entity.ok_or(rusqlite::Error::InvalidQuery)?,
        digest,
        bytes: row.get(4)?,
        adapter: row.get(5)?,
        status: row.get(6)?,
    })
}

/// Decodes one persisted project-entity row.
///
/// - `row`: SQLite row containing a project-entity record.
/// - `crate`: Internal visibility marker.
///
/// A decoded project-entity record.
///
/// Returns an identifier conversion error for malformed stored data.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn entity_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectEntityRecord> {
    let project: Vec<u8> = row.get(0)?;
    let entity: Vec<u8> = row.get(2)?;
    Ok(ProjectEntityRecord {
        project_id: fixed_id(project, "project", 0)?
            .ok_or(rusqlite::Error::InvalidQuery)
            .map(ProjectId::from_bytes)?,
        relative_path: row.get(1)?,
        entity: fixed_id(entity, "entity", 2)?
            .ok_or(rusqlite::Error::InvalidQuery)
            .map(EntityId::from_bytes)?,
        kind: row.get(3)?,
        name: row.get(4)?,
    })
}

/// Converts a SQLite blob into a fixed-width identifier when valid.
fn fixed_id<const N: usize>(
    bytes: Vec<u8>,
    description: &'static str,
    column: usize,
) -> rusqlite::Result<Option<[u8; N]>> {
    bytes.try_into().map(Some).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Blob,
            Box::new(std::io::Error::other(format!(
                "invalid {description} identifier"
            ))),
        )
    })
}
