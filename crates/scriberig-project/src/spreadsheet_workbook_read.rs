//! Reads exact-snapshot workbook and worksheet summaries.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_spreadsheet::Workbook;
use scriberig_store::{RevisionStore, SnapshotStore};

use super::{
    ProjectError, ProjectService, SpreadsheetWorkbookSummary, SpreadsheetWorksheetSummary,
    StoredWorksheetSummary, read_error,
};

impl ProjectService {
    /// Reads a workbook and its ordered worksheet summaries at an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Project owning the workbook.
    /// - `snapshot`: Exact immutable snapshot to inspect.
    /// - `workbook`: Stable semantic workbook identity.
    ///
    /// # Returns
    /// Workbook metadata with worksheet names and bounded used ranges.
    ///
    /// # Errors
    /// Returns an error when the workbook is absent, stale, or malformed.
    pub fn spreadsheet_workbook_summary(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        workbook: EntityId,
    ) -> Result<SpreadsheetWorkbookSummary, ProjectError> {
        let entities = self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_error)?;
        let workbook_entity = entities
            .iter()
            .find(|entity| entity.entity == workbook && entity.kind == "spreadsheet.workbook")
            .ok_or_else(|| {
                read_error("workbook_not_found", "workbook is not owned by the project")
            })?;
        let workbook_revision = self
            .store
            .resolve(snapshot, workbook)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                read_error(
                    "workbook_not_in_snapshot",
                    "workbook is absent from the requested snapshot",
                )
            })?;
        let revision = self
            .store
            .get(workbook_revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                read_error(
                    "workbook_revision_missing",
                    "workbook revision is unavailable",
                )
            })?;
        let semantic: Workbook = serde_json::from_slice(revision.payload()).map_err(|_| {
            read_error(
                "workbook_revision_invalid",
                "workbook semantic data is invalid",
            )
        })?;
        let mut worksheets = Vec::with_capacity(semantic.worksheets.len());
        for worksheet in semantic.worksheets {
            let entity = entities
                .iter()
                .find(|entity| {
                    entity.entity == worksheet
                        && entity.kind == "spreadsheet.worksheet"
                        && entity.relative_path == workbook_entity.relative_path
                })
                .ok_or_else(|| {
                    read_error(
                        "worksheet_not_found",
                        "workbook references a worksheet not owned by the project",
                    )
                })?;
            let revision_id = self
                .store
                .resolve(snapshot, worksheet)
                .map_err(crate::errors::storage_debug_error)?
                .ok_or_else(|| {
                    read_error(
                        "worksheet_not_in_snapshot",
                        "worksheet is absent from the requested snapshot",
                    )
                })?;
            let revision = self
                .store
                .get(revision_id)
                .map_err(crate::errors::storage_debug_error)?
                .ok_or_else(|| {
                    read_error(
                        "worksheet_revision_missing",
                        "worksheet revision is unavailable",
                    )
                })?;
            let metadata: StoredWorksheetSummary = serde_json::from_slice(revision.payload())
                .map_err(|_| {
                    read_error(
                        "worksheet_revision_invalid",
                        "worksheet metadata is invalid",
                    )
                })?;
            if metadata.id != worksheet || metadata.workbook != workbook {
                return Err(read_error(
                    "worksheet_owner_mismatch",
                    "worksheet metadata does not match its workbook collection",
                ));
            }
            worksheets.push(SpreadsheetWorksheetSummary {
                id: entity.entity,
                name: metadata.name,
                used_range: metadata.used_range,
                tables: metadata.tables,
            });
        }
        Ok(SpreadsheetWorkbookSummary {
            id: workbook,
            artifact_capability: self
                .artifact_capability_for_path(project, &workbook_entity.relative_path)?,
            snapshot,
            worksheets,
        })
    }
}
