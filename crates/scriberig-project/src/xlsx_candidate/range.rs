//! Builds bounded multi-cell spreadsheet candidate changes.

use super::{
    CellId, EntityId, ProjectCandidate, ProjectError, ProjectId, ProjectService,
    ProjectSourceChange, ProjectSourceChangeKind, SnapshotId, candidate_error, cell_value_at,
};
use scriberig_store::{RevisionStore, SnapshotStore};

impl ProjectService {
    /// Proposes a bounded exact-base edit to multiple cells on one worksheet.
    ///
    /// The candidate stores each changed cell's typed before/after values while
    /// the XLSX adapter rewrites the package once and verifies that no unrequested
    /// cell or opaque package part changed.
    ///
    /// # Parameters
    /// - `project`: Project owning the source workbook.
    /// - `base_snapshot`: Exact active semantic snapshot for the operation.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `edits`: One through five hundred distinct cell edits.
    ///
    /// # Returns
    /// Durable aggregate candidate with one semantic diff entry per requested cell.
    ///
    /// # Errors
    /// Returns stale-base, missing-target, read-only, invalid-edit, or persistence errors.
    pub fn propose_xlsx_range_edit(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        worksheet: EntityId,
        edits: &[scriberig_domain_xlsx::XlsxCellEdit],
    ) -> Result<ProjectCandidate, ProjectError> {
        if edits.is_empty() || edits.len() > scriberig_domain_xlsx::MAX_XLSX_CELL_EDITS {
            return Err(candidate_error(
                "invalid_cell_edit_count",
                "range edit must contain one to five hundred cells",
            ));
        }
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, worksheet)
            .map_err(crate::errors::storage_error)?
            .filter(|record| record.kind == "spreadsheet.worksheet")
            .ok_or_else(|| {
                candidate_error("worksheet_not_owned", "worksheet is not project-owned")
            })?;
        let revision_id = self
            .store
            .resolve(base_snapshot, worksheet)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                candidate_error("worksheet_not_found", "worksheet is absent from base")
            })?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                candidate_error(
                    "worksheet_revision_missing",
                    "worksheet revision is missing",
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
        let source = crate::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == ownership.relative_path && file.adapter == "xlsx")
            .ok_or_else(|| candidate_error("workbook_not_found", "worksheet source is not XLSX"))?;
        self.require_editable_source(project, &source.relative_path, source.digest)?;
        let updated = scriberig_domain_xlsx::edit_cells(&source.bytes, name, edits)
            .map_err(|error| candidate_error(error.code, &error.message))?;
        scriberig_domain_xlsx::validate_cell_edits(&source.bytes, &updated, name, edits)
            .map_err(|error| candidate_error(error.code, &error.message))?;
        let workbook = scriberig_domain_xlsx::semantic_workbook_id(source.entity);
        let semantic_changes = edits
            .iter()
            .map(|edit| {
                let target = CellId::new(workbook, worksheet, edit.row, edit.column).entity();
                Ok(crate::SemanticCandidateChange {
                    target,
                    property: "cell.value".to_owned(),
                    before: serde_json::to_value(cell_value_at(
                        &self.store,
                        base_snapshot,
                        target,
                    )?)
                    .map_err(|_| candidate_error("cell_diff_invalid", "prior value is invalid"))?,
                    after: serde_json::to_value(&edit.value).map_err(|_| {
                        candidate_error("cell_diff_invalid", "proposed value is invalid")
                    })?,
                })
            })
            .collect::<Result<Vec<_>, ProjectError>>()?;
        let mut candidate = self.propose_semantic_project_change(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "spreadsheet.set_range",
        )?;
        candidate.semantic_changes = semantic_changes;
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }
}
