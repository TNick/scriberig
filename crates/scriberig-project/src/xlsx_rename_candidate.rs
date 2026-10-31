//! Creates exact-base worksheet rename candidates through the shared change model.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{
    ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange, ProjectSourceChangeKind,
};

impl ProjectService {
    /// Proposes a loss-aware worksheet rename as a durable XLSX source candidate.
    ///
    /// The retained-package adapter updates supported formulas and defined-name
    /// references. Candidate creation never changes live package bytes.
    ///
    /// # Parameters
    /// - `project`: Project owning the workbook.
    /// - `base_snapshot`: Exact active semantic snapshot for the operation.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `new_name`: Requested authored worksheet name.
    ///
    /// # Returns
    /// Durable source candidate with exact old and renamed package bytes.
    ///
    /// # Errors
    /// Returns an error when the target is stale, missing, unowned, read-only, or invalid.
    pub fn propose_xlsx_sheet_rename(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        worksheet: EntityId,
        new_name: &str,
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
        let old_name = summary
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
        let updated = scriberig_domain_xlsx::rename_sheet(&source.bytes, old_name, new_name)
            .map_err(|error| candidate_error(error.code, &error.message))?;
        scriberig_domain_xlsx::validate_edit(&source.bytes, &updated)
            .map_err(|error| candidate_error(error.code, &error.message))?;
        self.propose_semantic_project_change(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "spreadsheet.rename_sheet",
        )
    }
}

/// Creates a structured XLSX rename error without exposing source bytes.
///
/// # Parameters
/// - `code`: Stable error category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// A project error suitable for application and protocol surfaces.
fn candidate_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
