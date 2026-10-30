//! Creates exact-base `XLSX` cell-edit candidates without changing live files.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_spreadsheet::{Cell, CellId, CellValue};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{
    ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange, ProjectSourceChangeKind,
};

#[path = "xlsx_candidate/range.rs"]
mod range;

impl ProjectService {
    /// Proposes a cell edit using the exact semantic cell handle.
    ///
    /// The cell and its owning worksheet are resolved from the supplied
    /// snapshot before delegating to the shared exact-base candidate path.
    ///
    /// # Parameters
    /// - `project`: Project owning the source workbook.
    /// - `base_snapshot`: Exact active semantic snapshot for the operation.
    /// - `cell`: Stable semantic identity of the target cell.
    /// - `value`: New typed cell value.
    ///
    /// # Returns
    /// Durable source candidate for the exact target cell.
    ///
    /// # Errors
    /// Returns an error when the cell is stale, missing, unowned, read-only, or invalid.
    pub fn propose_xlsx_semantic_cell_edit(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        cell: EntityId,
        value: &CellValue,
    ) -> Result<ProjectCandidate, ProjectError> {
        let ownership = self
            .store
            .project_entity(project, cell)
            .map_err(crate::errors::storage_error)?
            .filter(|record| record.kind == "spreadsheet.cell")
            .ok_or_else(|| candidate_error("cell_not_owned", "target is not a project cell"))?;
        let revision_id = self
            .store
            .resolve(base_snapshot, cell)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| candidate_error("cell_not_in_snapshot", "cell is absent from base"))?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| candidate_error("cell_revision_missing", "cell revision is missing"))?;
        let semantic: Cell = serde_json::from_slice(revision.payload())
            .map_err(|_| candidate_error("cell_invalid", "cell revision is malformed"))?;
        if semantic.id.entity() != cell {
            return Err(candidate_error(
                "cell_identity_mismatch",
                "cell revision identity does not match the requested handle",
            ));
        }
        let worksheet = self
            .store
            .project_entity(project, semantic.worksheet)
            .map_err(crate::errors::storage_error)?
            .filter(|record| {
                record.kind == "spreadsheet.worksheet"
                    && record.relative_path == ownership.relative_path
            })
            .ok_or_else(|| {
                candidate_error(
                    "cell_worksheet_mismatch",
                    "target cell is not owned by a worksheet in the same workbook",
                )
            })?;
        let candidate = self.propose_xlsx_cell_edit(
            project,
            base_snapshot,
            worksheet.entity,
            semantic.row,
            semantic.column,
            value,
        )?;
        if candidate.path != ownership.relative_path {
            return Err(candidate_error(
                "cell_source_mismatch",
                "target cell candidate resolved to a different workbook source",
            ));
        }
        Ok(candidate)
    }

    /// Proposes one sparse `XLSX` cell edit as a durable exact-base candidate.
    ///
    /// The target worksheet must resolve in the requested active snapshot. The
    /// adapter writes into retained package bytes and the shared candidate
    /// service records an update effect; this method never changes the file.
    ///
    /// # Parameters
    /// - `project`: Project owning the source workbook.
    /// - `base_snapshot`: Exact active semantic snapshot for the operation.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `row`: One-based worksheet row.
    /// - `column`: One-based worksheet column.
    /// - `value`: New typed cell value.
    ///
    /// # Returns
    /// Durable source candidate with exact old/new `XLSX` bytes.
    ///
    /// # Errors
    /// Returns an error when the target is stale, missing, read-only, or invalid.
    pub fn propose_xlsx_cell_edit(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        worksheet: EntityId,
        row: u32,
        column: u32,
        value: &CellValue,
    ) -> Result<ProjectCandidate, ProjectError> {
        let workspace = self.workspace(project)?;
        if workspace.snapshot != base_snapshot {
            return Err(candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let revision_id = self
            .store
            .resolve(base_snapshot, worksheet)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                candidate_error(
                    "worksheet_not_found",
                    "worksheet is absent from the base snapshot",
                )
            })?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                candidate_error(
                    "worksheet_revision_missing",
                    "worksheet revision is unavailable",
                )
            })?;
        let summary: serde_json::Value =
            serde_json::from_slice(revision.payload()).map_err(|_| {
                candidate_error("worksheet_summary_invalid", "worksheet metadata is invalid")
            })?;
        let name = summary
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                candidate_error("worksheet_summary_invalid", "worksheet name is missing")
            })?;
        let ownership = self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_error)?
            .into_iter()
            .find(|entry| entry.entity == worksheet && entry.kind == "spreadsheet.worksheet")
            .ok_or_else(|| {
                candidate_error(
                    "worksheet_not_owned",
                    "worksheet is not owned by this project",
                )
            })?;
        let source = crate::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == ownership.relative_path && file.adapter == "xlsx")
            .ok_or_else(|| {
                candidate_error(
                    "workbook_not_found",
                    "worksheet source is not an XLSX artifact",
                )
            })?;
        self.require_editable_source(project, &source.relative_path, source.digest)?;
        let updated = scriberig_domain_xlsx::edit_cell(&source.bytes, name, row, column, value)
            .map_err(|error| candidate_error(error.code, &error.message))?;
        scriberig_domain_xlsx::validate_cell_edit(
            &source.bytes,
            &updated,
            name,
            row,
            column,
            value,
        )
        .map_err(|error| candidate_error(error.code, &error.message))?;
        let target = CellId::new(
            scriberig_domain_xlsx::semantic_workbook_id(source.entity),
            worksheet,
            row,
            column,
        )
        .entity();
        let before = cell_value_at(&self.store, base_snapshot, target)?;
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "spreadsheet.set_cell",
            target,
        )?;
        self.persist_xlsx_cell_diff(&mut candidate, target, before, value)?;
        Ok(candidate)
    }

    /// Adds exact before/after cell values and persists the candidate.
    ///
    /// # Parameters
    /// - `candidate`: Staged candidate to complete and persist.
    /// - `target`: Stable semantic cell identity.
    /// - `before`: Cell value resolved from the base snapshot.
    /// - `after`: Proposed typed cell value.
    ///
    /// # Returns
    /// The persisted candidate for shared validation and application.
    ///
    /// # Errors
    /// Returns candidate, serialization, or storage errors.
    fn persist_xlsx_cell_diff(
        &self,
        candidate: &mut ProjectCandidate,
        target: EntityId,
        before: CellValue,
        after: &CellValue,
    ) -> Result<(), ProjectError> {
        candidate
            .semantic_changes
            .push(crate::SemanticCandidateChange {
                target,
                property: "cell.value".to_owned(),
                before: serde_json::to_value(before).map_err(|_| {
                    candidate_error("cell_diff_invalid", "prior cell value is invalid")
                })?,
                after: serde_json::to_value(after).map_err(|_| {
                    candidate_error("cell_diff_invalid", "proposed cell value is invalid")
                })?,
            });
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(())
    }
}

/// Reads an exact-snapshot cell value, treating sparse absence as blank.
///
/// # Parameters
/// - `store`: Project semantic revision store.
/// - `snapshot`: Exact immutable snapshot to read.
/// - `cell`: Stable semantic cell identity.
///
/// # Returns
/// The typed cell value or `Blank` when no sparse record exists.
///
/// # Errors
/// Returns an error when an existing cell revision is missing, malformed, or mismatched.
fn cell_value_at(
    store: &scriberig_store_sqlite::SqliteStore,
    snapshot: SnapshotId,
    cell: EntityId,
) -> Result<CellValue, ProjectError> {
    let Some(revision) = store
        .resolve(snapshot, cell)
        .map_err(crate::errors::storage_debug_error)?
    else {
        return Ok(CellValue::Blank);
    };
    let record = store
        .get(revision)
        .map_err(crate::errors::storage_debug_error)?
        .ok_or_else(|| candidate_error("cell_revision_missing", "cell revision is missing"))?;
    let semantic: Cell = serde_json::from_slice(record.payload())
        .map_err(|_| candidate_error("cell_invalid", "cell revision is malformed"))?;
    if semantic.id.entity() != cell {
        return Err(candidate_error(
            "cell_identity_mismatch",
            "cell revision identity does not match its handle",
        ));
    }
    Ok(semantic.value)
}

/// Creates a structured `XLSX` candidate error.
///
/// # Parameters
/// - `code`: Stable error category.
/// - `message`: Safe diagnostic detail.
///
/// A project error without source content.
/// # Returns
/// The result of the requested operation.
fn candidate_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
