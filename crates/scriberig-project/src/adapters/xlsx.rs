//! Imports semantic workbook, worksheet, and sparse cell entities.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_spreadsheet::{CellIndex, CellIndexEntry, Range, Table};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};
use serde::Serialize;

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

/// Imports sparse workbook, worksheet, and cell semantic entities.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted XLSX source file.
/// - `source`: Exact source artifact entity.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Returns
/// Unit after all supported workbook semantics are appended.
///
/// # Errors
/// Returns XLSX parser, serialization, or revision diagnostics.
pub(crate) fn import_xlsx(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let imported = scriberig_domain_xlsx::import_xlsx(
        &file.bytes,
        scriberig_domain_xlsx::semantic_workbook_id(source),
    )
    .map_err(|error| {
        adapter_diagnostics_error(file, "parse", error.code, &[error.message], true)
    })?;
    let workbook_payload = serde_json::to_vec(&imported.workbook).map_err(|error| {
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
        imported.workbook.id,
        "spreadsheet.workbook",
        workbook_payload,
        revisions,
        changes,
        ownership,
    )?;
    for worksheet in &imported.worksheets {
        let cell_index = CellIndex::new(worksheet.cells.iter().map(|cell| CellIndexEntry {
            entity: cell.id.entity(),
            row: cell.row,
            column: cell.column,
        }))
        .map_err(|error| {
            adapter_diagnostics_error(
                file,
                "index",
                "cell_index_invalid",
                &[format!("worksheet cell index is invalid: {error:?}")],
                true,
            )
        })?;
        let worksheet_payload = serde_json::to_vec(&WorksheetSummary {
            id: worksheet.id,
            workbook: worksheet.workbook,
            name: &worksheet.name,
            used_range: &worksheet.used_range,
            tables: &worksheet.tables,
            cell_index: &cell_index,
        })
        .map_err(|error| {
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
            worksheet.id,
            "spreadsheet.worksheet",
            worksheet_payload,
            revisions,
            changes,
            ownership,
        )?;
        for cell in worksheet.cells.iter() {
            let cell_payload = serde_json::to_vec(cell).map_err(|error| {
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
                cell.id.entity(),
                "spreadsheet.cell",
                cell_payload,
                revisions,
                changes,
                ownership,
            )?;
        }
    }
    Ok(())
}

/// Stores bounded worksheet metadata separately from its sparse cell entities.
#[derive(Serialize)]
struct WorksheetSummary<'a> {
    /// Stable worksheet semantic identity.
    id: scriberig_core::EntityId,
    /// Owning workbook semantic identity.
    workbook: scriberig_core::EntityId,
    /// Authored worksheet name.
    name: &'a str,
    /// Bounded used range, when reported by the source package.
    used_range: &'a Option<Range>,
    /// Named worksheet tables and their semantic ranges.
    tables: &'a [Table],
    /// Snapshot-local membership handles ordered by worksheet coordinates.
    cell_index: &'a CellIndex,
}
