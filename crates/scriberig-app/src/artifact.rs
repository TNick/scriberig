//! Exposes bounded, authorized artifact inspection through the shared application service.

use scriberig_core::EntityId;
use scriberig_core::ProjectId;
use scriberig_domain_spreadsheet::CellRange;
use scriberig_project::{ArtifactManifest, ArtifactSynchronizationStatus, CandidateValidation};

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

#[path = "artifact/spreadsheet_edit.rs"]
mod spreadsheet_edit;
#[path = "artifact/spreadsheet_find.rs"]
mod spreadsheet_find;
pub use spreadsheet_find::SpreadsheetCellSearchRequest;

/// Maximum number of artifact summaries returned by one application request.
pub const MAX_ARTIFACT_PAGE: usize = 100;

/// Contains a bounded sparse worksheet cell page from an exact snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCells {
    /// Project whose semantic worksheet is read.
    pub project: ProjectId,
    /// Exact semantic snapshot used to resolve cell revisions.
    pub snapshot: scriberig_core::SnapshotId,
    /// Bounded sparse cell page.
    pub page: scriberig_project::SpreadsheetCellPage,
}

/// Contains one exact-coordinate cell inspection, including absent blank cells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellInspection {
    /// Project owning the semantic worksheet.
    pub project: ProjectId,
    /// Exact semantic snapshot used by the inspection.
    pub snapshot: scriberig_core::SnapshotId,
    /// Stable semantic worksheet identity.
    pub worksheet: EntityId,
    /// One-based row coordinate.
    pub row: u32,
    /// One-based column coordinate.
    pub column: u32,
    /// Semantic cell when non-empty; absent cells are blank by sparse schema.
    pub cell: Option<scriberig_domain_spreadsheet::Cell>,
}

/// Contains a durable, reviewable spreadsheet source candidate without exposing source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetEditCandidate {
    /// Stable project candidate identity.
    pub id: String,
    /// Project owning the candidate.
    pub project: ProjectId,
    /// Exact snapshot against which the edit was proposed.
    pub base_snapshot: scriberig_core::SnapshotId,
    /// Project-relative workbook path.
    pub path: String,
    /// Current validation state of the candidate.
    pub validation: CandidateValidation,
}

/// Describes one typed spreadsheet cell update in a bounded range proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellEdit {
    /// One-based worksheet row.
    pub row: u32,
    /// One-based worksheet column.
    pub column: u32,
    /// New authored semantic value or formula.
    pub value: scriberig_domain_spreadsheet::CellValue,
}

/// Contains a bounded page of logical artifact manifests for one project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactManifestPage {
    /// Project whose persisted scan was summarized.
    pub project: ProjectId,
    /// Bounded logical artifacts in deterministic key order.
    pub artifacts: Vec<ArtifactManifest>,
    /// Live synchronization status for each returned artifact, in the same order.
    pub synchronization: Vec<ArtifactSynchronizationStatus>,
    /// Number of artifacts in the persisted scan.
    pub total: usize,
    /// Offset to request for the next page.
    pub next_offset: Option<usize>,
}

impl<B> ApplicationService<B> {
    /// Inspects one cell coordinate under an exact snapshot and Inspect grant.
    ///
    /// # Parameters
    /// - `context`: Authorized request context bound to the exact snapshot.
    /// - `project`: Project owning the worksheet.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `row`: One-based target row.
    /// - `column`: One-based target column.
    ///
    /// # Returns
    /// Semantic cell content or an absent value for a sparse blank coordinate.
    ///
    /// # Errors
    /// Returns authorization, request, storage, or semantic decoding failures.
    pub fn inspect_spreadsheet_cell(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        worksheet: EntityId,
        row: u32,
        column: u32,
    ) -> Result<SpreadsheetCellInspection, ApplicationError> {
        let range = CellRange {
            first_row: row,
            first_column: column,
            last_row: row,
            last_column: column,
        };
        let page = self.read_spreadsheet_cells(context, project, worksheet, Some(&range), 0, 1)?;
        Ok(SpreadsheetCellInspection {
            project,
            snapshot: page.snapshot,
            worksheet,
            row,
            column,
            cell: page.page.cells.into_iter().next(),
        })
    }

    /// Reads workbook and worksheet summaries under an exact Inspect grant.
    ///
    /// # Parameters
    /// - `context`: Authorized request context bound to the exact snapshot.
    /// - `project`: Project owning the workbook.
    /// - `workbook`: Stable semantic workbook identity.
    ///
    /// # Returns
    /// Workbook and ordered worksheet summaries from the requested snapshot.
    ///
    /// # Errors
    /// Returns authorization, request, storage, or semantic decoding failures.
    pub fn inspect_spreadsheet_workbook(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        workbook: EntityId,
    ) -> Result<scriberig_project::SpreadsheetWorkbookSummary, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "spreadsheet.workbook.inspect".into(),
                message: "spreadsheet inspection requires an exact snapshot".into(),
            })?;
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
                operation: "spreadsheet.workbook.inspect".into(),
            }
        })?;
        service
            .spreadsheet_workbook_summary(project, snapshot, workbook)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "spreadsheet.workbook.inspect".into(),
                message: format!("{}: {}", error.code, error.message),
            })
    }

    /// Proposes a semantic worksheet rename as a durable source candidate.
    ///
    /// # Parameters
    /// - `context`: Request context with an exact snapshot and scoped Propose grant.
    /// - `project`: Project owning the workbook.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `new_name`: Requested authored worksheet name.
    ///
    /// # Returns
    /// A candidate handle suitable for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns authorization, stale-base, unsupported-adapter, or candidate errors.
    pub fn propose_spreadsheet_sheet_rename(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        worksheet: EntityId,
        new_name: &str,
    ) -> Result<SpreadsheetEditCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "spreadsheet.sheet.rename".into(),
                message: "spreadsheet renames require an exact snapshot".into(),
            })?;
        spreadsheet_edit::authorize_spreadsheet_proposal(context, project, worksheet, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.sheet.rename".into(),
            }
        })?;
        let candidate = service
            .propose_xlsx_sheet_rename(project, snapshot, worksheet, new_name)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "spreadsheet.sheet.rename".into(),
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

    /// Reads a bounded sparse worksheet page under an explicit Inspect grant.
    ///
    /// # Parameters
    /// - `context`: Authorized request context bound to an exact snapshot.
    /// - `project`: Project owning the worksheet.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `range`: Optional inclusive rectangular bounds within the worksheet.
    /// - `offset`: Zero-based cell offset in row/column order.
    /// - `limit`: Maximum of five hundred sparse cells.
    ///
    /// # Returns
    /// Authorized, snapshot-bound sparse cells with continuation metadata.
    ///
    /// # Errors
    /// Returns an authorization, request, storage, or semantic decoding error.
    pub fn read_spreadsheet_cells(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        worksheet: EntityId,
        range: Option<&CellRange>,
        offset: usize,
        limit: usize,
    ) -> Result<SpreadsheetCells, ApplicationError> {
        if !(1..=scriberig_project::MAX_SPREADSHEET_CELL_PAGE).contains(&limit) {
            return Err(ApplicationError::InvalidRequest {
                operation: "spreadsheet.cells.read".into(),
                message: "spreadsheet page limit must be between one and five hundred".into(),
            });
        }
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "spreadsheet.cells.read".into(),
                message: "spreadsheet reads require an exact snapshot".into(),
            })?;
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
                operation: "spreadsheet.cells.read".into(),
            }
        })?;
        let page = service
            .spreadsheet_cell_page(project, snapshot, worksheet, range, offset, limit)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "spreadsheet.cells.read".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(SpreadsheetCells {
            project,
            snapshot,
            page,
        })
    }

    /// Lists a bounded page of artifacts from the persisted project scan.
    ///
    /// The operation requires an Inspect grant scoped to this project. It reads
    /// exact scan descriptors from the durable catalog without reclassifying files.
    /// It hashes only the returned artifact members to report live synchronization
    /// against the persisted materialization baseline.
    ///
    /// # Parameters
    /// - `context`: Authorized application request context.
    /// - `project`: Project to inspect.
    /// - `offset`: Zero-based artifact offset.
    /// - `limit`: Maximum number of summaries, between one and one hundred.
    ///
    /// # Returns
    /// A deterministic bounded page of artifact summaries.
    ///
    /// # Errors
    /// Returns authorization, request, catalog, or availability errors.
    pub fn inspect_artifacts(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        offset: usize,
        limit: usize,
    ) -> Result<ArtifactManifestPage, ApplicationError> {
        if !(1..=MAX_ARTIFACT_PAGE).contains(&limit) {
            return Err(ApplicationError::InvalidRequest {
                operation: "artifact.inspect".into(),
                message: "artifact page limit must be between one and one hundred".into(),
            });
        }
        let scope = CapabilityScope::Custom(format!("project:{project}"));
        if !context.capabilities.permits(
            &CapabilityAction::Inspect,
            &scope,
            context.snapshot,
            Some(OperationKind::Inspect),
        ) {
            return Err(ApplicationError::PermissionDenied {
                action: "Inspect".into(),
                scope: format!("project:{project}"),
            });
        }
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "artifact.inspect".into(),
            }
        })?;
        let all = service
            .artifacts(project)
            .map_err(|error| ApplicationError::Internal {
                message: format!("project artifact catalog failed: {}", error.code),
            })?;
        if offset > all.len() {
            return Err(ApplicationError::InvalidRequest {
                operation: "artifact.inspect".into(),
                message: "artifact offset exceeds the available result count".into(),
            });
        }
        let total = all.len();
        let artifacts = all.into_iter().skip(offset).take(limit).collect::<Vec<_>>();
        let synchronization = service
            .artifact_synchronization(project, &artifacts)
            .map_err(|error| ApplicationError::Internal {
                message: format!("artifact synchronization inspection failed: {}", error.code),
            })?;
        let next_offset = (offset + artifacts.len() < total).then_some(offset + artifacts.len());
        Ok(ArtifactManifestPage {
            project,
            artifacts,
            synchronization,
            total,
            next_offset,
        })
    }
}
