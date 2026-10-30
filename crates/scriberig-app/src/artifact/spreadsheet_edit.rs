//! Authorizes spreadsheet candidate edits through shared semantic operations.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_spreadsheet::CellValue;

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

use super::SpreadsheetEditCandidate;

impl<B> ApplicationService<B> {
    /// Proposes up to five hundred typed cell edits as one exact-base candidate.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and scoped Propose grant.
    /// - `project`: Project owning the workbook.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `edits`: Distinct one-based coordinates and typed values.
    ///
    /// # Returns
    /// One candidate handle suitable for shared validation and review.
    ///
    /// # Errors
    /// Returns authorization, stale-base, unsupported-adapter, invalid-edit, or candidate errors.
    pub fn propose_spreadsheet_range_edit(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        worksheet: EntityId,
        edits: &[super::SpreadsheetCellEdit],
    ) -> Result<SpreadsheetEditCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "spreadsheet.range.propose".into(),
                message: "spreadsheet edits require an exact snapshot".into(),
            })?;
        authorize_spreadsheet_proposal(context, project, worksheet, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.range.propose".into(),
            }
        })?;
        let domain_edits = edits
            .iter()
            .map(|edit| scriberig_domain_xlsx::XlsxCellEdit {
                row: edit.row,
                column: edit.column,
                value: edit.value.clone(),
            })
            .collect::<Vec<_>>();
        let candidate = service
            .propose_xlsx_range_edit(project, snapshot, worksheet, &domain_edits)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "spreadsheet.range.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(SpreadsheetEditCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }

    /// Proposes a semantic cell edit as a durable candidate without mutating the live workbook.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and scoped Propose grant.
    /// - `project`: Project owning the workbook.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `row`: One-based worksheet row.
    /// - `column`: One-based worksheet column.
    /// - `value`: Typed semantic cell value or formula.
    ///
    /// # Returns
    /// A candidate handle suitable for shared validation and review.
    ///
    /// # Errors
    /// Returns authorization, stale-base, unsupported-adapter, or candidate errors.
    pub fn propose_spreadsheet_cell_edit(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        worksheet: EntityId,
        row: u32,
        column: u32,
        value: &CellValue,
    ) -> Result<SpreadsheetEditCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "spreadsheet.cell.propose".into(),
                message: "spreadsheet edits require an exact snapshot".into(),
            })?;
        authorize_spreadsheet_proposal(context, project, worksheet, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.cell.propose".into(),
            }
        })?;
        let candidate = service
            .propose_xlsx_cell_edit(project, snapshot, worksheet, row, column, value)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "spreadsheet.cell.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(SpreadsheetEditCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }

    /// Proposes a spreadsheet edit against one exact semantic cell handle.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and scoped Propose grant.
    /// - `project`: Project owning the workbook.
    /// - `cell`: Stable semantic identity of the target cell.
    /// - `value`: Typed semantic cell value or formula.
    ///
    /// # Returns
    /// A candidate handle suitable for shared validation and review.
    ///
    /// # Errors
    /// Returns authorization, stale-base, unsupported-adapter, or candidate errors.
    pub fn propose_spreadsheet_semantic_cell_edit(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        cell: EntityId,
        value: &CellValue,
    ) -> Result<SpreadsheetEditCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "spreadsheet.cell.propose".into(),
                message: "spreadsheet edits require an exact snapshot".into(),
            })?;
        authorize_spreadsheet_proposal(context, project, cell, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.cell.propose".into(),
            }
        })?;
        let candidate = service
            .propose_xlsx_semantic_cell_edit(project, snapshot, cell, value)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "spreadsheet.cell.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(SpreadsheetEditCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            path: candidate.path,
            validation: candidate.validation,
        })
    }
}

/// Authorizes a spreadsheet mutation at its exact worksheet or cell target.
///
/// # Parameters
/// - `context`: Request context containing scoped Propose authority.
/// - `project`: Project owning the spreadsheet target.
/// - `target`: Exact worksheet or cell semantic identity.
/// - `snapshot`: Exact candidate base snapshot.
///
/// # Returns
/// Unit when target-scoped or compatibility project-scoped authority applies.
///
/// # Errors
/// Returns a permission error when neither scope authorizes the mutation.
pub(super) fn authorize_spreadsheet_proposal(
    context: &ApplicationContext,
    project: ProjectId,
    target: EntityId,
    snapshot: scriberig_core::SnapshotId,
) -> Result<(), ApplicationError> {
    let target_scope = CapabilityScope::Entity(target);
    let project_scope = CapabilityScope::Custom(format!("project:{project}"));
    let operation = Some(OperationKind::DomainOperation);
    if context.capabilities.permits(
        &CapabilityAction::Propose,
        &target_scope,
        Some(snapshot),
        operation,
    ) || context.capabilities.permits(
        &CapabilityAction::Propose,
        &project_scope,
        Some(snapshot),
        operation,
    ) {
        return Ok(());
    }
    Err(ApplicationError::PermissionDenied {
        action: "Propose".into(),
        scope: format!("spreadsheet target {target} in project {project}"),
    })
}
