//! Authorizes bounded exact-snapshot spreadsheet cell search.

use scriberig_core::{EntityId, ProjectId, SnapshotId};

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Identifies a workbook search at one exact project snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellSearchRequest {
    /// Project owning the workbook.
    pub project: ProjectId,
    /// Exact immutable snapshot to search.
    pub snapshot: SnapshotId,
    /// Stable semantic workbook identity.
    pub workbook: EntityId,
    /// Non-empty authored-value substring to find.
    pub query: String,
    /// Zero-based result offset.
    pub offset: usize,
    /// Maximum number of matches to return.
    pub limit: usize,
}

impl<B> ApplicationService<B> {
    /// Finds authored spreadsheet cell content through project-scoped Inspect authority.
    ///
    /// # Parameters
    /// - `context`: Caller capability context bound to the exact requested snapshot.
    /// - `request`: Project, snapshot, workbook, query, and page bounds.
    ///
    /// # Returns
    /// Bounded cell matches with stable worksheet and cell identities.
    ///
    /// # Errors
    /// Returns authorization, availability, stale-snapshot, or project search errors.
    pub fn find_spreadsheet_cells(
        &self,
        context: &ApplicationContext,
        request: &SpreadsheetCellSearchRequest,
    ) -> Result<scriberig_project::SpreadsheetCellSearch, ApplicationError> {
        let SpreadsheetCellSearchRequest {
            project,
            snapshot,
            workbook,
            query,
            offset,
            limit,
        } = request;
        let (project, snapshot, workbook) = (*project, *snapshot, *workbook);
        if context.snapshot != Some(snapshot) {
            return Err(ApplicationError::InvalidRequest {
                operation: "spreadsheet.cell.find".to_owned(),
                message: "spreadsheet search requires the exact requested snapshot".into(),
            });
        }
        let scope = CapabilityScope::Custom(format!("project:{project}"));
        if !context.capabilities.permits(
            &CapabilityAction::Inspect,
            &scope,
            Some(snapshot),
            Some(OperationKind::Inspect),
        ) {
            return Err(ApplicationError::PermissionDenied {
                action: "Inspect".into(),
                scope: format!("project:{project}"),
            });
        }
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.cell.find".into(),
            }
        })?;
        service
            .find_spreadsheet_cells(project, snapshot, workbook, query, *offset, *limit)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "spreadsheet.cell.find".into(),
                message: format!("{}: {}", error.code, error.message),
            })
    }
}
