//! Creates exact-base `DOCX` paragraph candidates through the shared project change path.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_document::Paragraph;
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::docx_candidate_helpers::{
    find_table_cell, single_cell_paragraph, table_cell_containing_paragraph,
};
use crate::{
    ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange, ProjectSourceChangeKind,
};

impl ProjectService {
    /// Proposes replacement of a simple DOCX table cell by its stable cell handle.
    ///
    /// A cell must contain exactly one paragraph and no nested table; more
    /// complex cells are rejected until their structured editing contract exists.
    ///
    /// # Parameters
    /// - `project`: Project owning the document source.
    /// - `base_snapshot`: Exact current project snapshot.
    /// - `cell`: Stable semantic table-cell identity.
    /// - `replacement`: Replacement UTF-8 cell text.
    ///
    /// # Returns
    /// Durable exact-base source candidate for validation and shared apply.
    ///
    /// # Errors
    /// Returns an error for stale snapshots, unsupported cell structure, unsafe
    /// DOCX source, missing targets, or candidate persistence failures.
    pub fn propose_docx_table_cell_edit(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        cell: EntityId,
        replacement: &str,
    ) -> Result<ProjectCandidate, ProjectError> {
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(docx_candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, cell)
            .map_err(crate::errors::storage_error)?
            .filter(|entity| entity.kind == "document.table_cell")
            .ok_or_else(|| docx_candidate_error("cell_not_found", "target is not an owned cell"))?;
        self.store
            .resolve(base_snapshot, cell)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                docx_candidate_error("cell_not_in_snapshot", "cell is absent from base")
            })?;
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == ownership.relative_path && file.adapter == "docx")
            .ok_or_else(|| docx_candidate_error("document_not_found", "cell source is not DOCX"))?;
        self.require_editable_source(project, &source.relative_path, source.digest)?;
        let document_id = scriberig_domain_docx::semantic_document_id(source.entity);
        let imported = scriberig_domain_docx::import_docx(&source.bytes, document_id)
            .map_err(|message| docx_candidate_error("docx_parse_failed", &message))?;
        let semantic_cell = find_table_cell(&imported.document, cell)
            .ok_or_else(|| docx_candidate_error("cell_not_found", "cell is absent from source"))?;
        let paragraph = single_cell_paragraph(semantic_cell)?;
        let before = paragraph
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>();
        let updated = scriberig_domain_docx::replace_simple_paragraph(
            &source.bytes,
            document_id,
            paragraph.id,
            replacement,
        )
        .map_err(|message| docx_candidate_error("docx_edit_unsupported", &message))?;
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "document.update_table_cell",
            cell,
        )?;
        candidate
            .semantic_changes
            .push(crate::SemanticCandidateChange {
                target: cell,
                property: "table_cell.text".to_owned(),
                before: serde_json::json!(before),
                after: serde_json::json!(replacement),
            });
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }

    /// Proposes a safe paragraph append without changing the live `DOCX` file.
    ///
    /// # Parameters
    /// - `project`: Project owning the document source.
    /// - `base_snapshot`: Exact current project snapshot.
    /// - `section`: Stable semantic section identity receiving the appended paragraph.
    /// - `text`: New UTF-8 paragraph text.
    ///
    /// # Returns
    /// Durable exact-base source candidate for validation and shared apply.
    ///
    /// # Errors
    /// Returns an error for stale snapshots, non-section targets, unsafe `DOCX`
    /// structures, unsupported source features, or candidate persistence failures.
    pub fn propose_docx_paragraph_insert(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        section: EntityId,
        text: &str,
    ) -> Result<ProjectCandidate, ProjectError> {
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(docx_candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, section)
            .map_err(crate::errors::storage_error)?
            .filter(|entity| entity.kind == "document.section")
            .ok_or_else(|| {
                docx_candidate_error(
                    "section_not_found",
                    "target is not an owned document section",
                )
            })?;
        self.store
            .resolve(base_snapshot, section)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                docx_candidate_error("section_not_in_snapshot", "section is absent from base")
            })?;
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == ownership.relative_path && file.adapter == "docx")
            .ok_or_else(|| {
                docx_candidate_error("document_not_found", "section source is not DOCX")
            })?;
        self.require_editable_source(project, &source.relative_path, source.digest)?;
        let document_id = scriberig_domain_docx::semantic_document_id(source.entity);
        let updated = scriberig_domain_docx::append_simple_paragraph(
            &source.bytes,
            document_id,
            section,
            text,
        )
        .map_err(|message| docx_candidate_error("docx_edit_unsupported", &message))?;
        self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "document.insert_paragraph",
            section,
        )
    }

    /// Proposes a safe paragraph replacement without changing the live `DOCX` file.
    ///
    /// # Parameters
    /// - `project`: Project owning the document source.
    /// - `base_snapshot`: Exact current project snapshot.
    /// - `paragraph`: Stable semantic paragraph identity.
    /// - `replacement`: Replacement UTF-8 paragraph text.
    ///
    /// # Returns
    /// Durable exact-base source candidate for validation and shared apply.
    ///
    /// # Errors
    /// Returns an error for stale snapshots, non-paragraph targets, unsafe `DOCX`
    /// structures, unsupported source features, or candidate persistence failures.
    pub fn propose_docx_paragraph_edit(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        paragraph: EntityId,
        replacement: &str,
    ) -> Result<ProjectCandidate, ProjectError> {
        if self.workspace(project)?.snapshot != base_snapshot {
            return Err(docx_candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, paragraph)
            .map_err(crate::errors::storage_error)?
            .filter(|entity| entity.kind == "document.paragraph")
            .ok_or_else(|| {
                docx_candidate_error("paragraph_not_found", "target is not an owned paragraph")
            })?;
        self.store
            .resolve(base_snapshot, paragraph)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                docx_candidate_error("paragraph_not_in_snapshot", "paragraph is absent from base")
            })?;
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| file.relative_path == ownership.relative_path && file.adapter == "docx")
            .ok_or_else(|| {
                docx_candidate_error("document_not_found", "paragraph source is not DOCX")
            })?;
        self.require_editable_source(project, &source.relative_path, source.digest)?;
        let document_id = scriberig_domain_docx::semantic_document_id(source.entity);
        let updated = scriberig_domain_docx::replace_simple_paragraph(
            &source.bytes,
            document_id,
            paragraph,
            replacement,
        )
        .map_err(|message| docx_candidate_error("docx_edit_unsupported", &message))?;
        let semantic_paragraph = self.paragraph_at(base_snapshot, paragraph)?;
        let before = semantic_paragraph
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>();
        let imported = scriberig_domain_docx::import_docx(&source.bytes, document_id)
            .map_err(|message| docx_candidate_error("docx_parse_failed", &message))?;
        let table_cell = table_cell_containing_paragraph(&imported.document, paragraph);
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "document.replace_paragraph",
            paragraph,
        )?;
        candidate
            .semantic_changes
            .push(crate::SemanticCandidateChange {
                target: paragraph,
                property: "paragraph.text".to_owned(),
                before: serde_json::json!(before),
                after: serde_json::json!(replacement),
            });
        if let Some(cell) = table_cell {
            candidate
                .semantic_changes
                .push(crate::SemanticCandidateChange {
                    target: cell,
                    property: "table_cell.paragraph_text".to_owned(),
                    before: serde_json::json!(before),
                    after: serde_json::json!(replacement),
                });
        }
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }

    /// Resolves and decodes one paragraph revision from the exact source snapshot.
    ///
    /// # Parameters
    /// - `snapshot`: Immutable snapshot containing the paragraph.
    /// - `paragraph`: Stable paragraph identity to resolve.
    ///
    /// # Returns
    /// The exact semantic paragraph stored at the snapshot.
    ///
    /// # Errors
    /// Returns an error when the revision is absent, unreadable, or malformed.
    fn paragraph_at(
        &self,
        snapshot: SnapshotId,
        paragraph: EntityId,
    ) -> Result<Paragraph, ProjectError> {
        let revision = self
            .store
            .resolve(snapshot, paragraph)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                docx_candidate_error("paragraph_not_in_snapshot", "paragraph is absent from base")
            })?;
        let revision = self
            .store
            .get(revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                docx_candidate_error(
                    "paragraph_revision_missing",
                    "paragraph revision is missing",
                )
            })?;
        serde_json::from_slice(revision.payload()).map_err(|_| {
            docx_candidate_error("paragraph_invalid", "paragraph revision is malformed")
        })
    }
}

/// Creates a structured `DOCX` candidate failure.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe diagnostic without source bytes.
///
/// Project error used by the shared candidate boundary.
/// # Returns
/// The result of the requested operation.
pub(crate) fn docx_candidate_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
