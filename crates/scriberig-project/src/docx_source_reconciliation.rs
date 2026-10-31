//! Guards external DOCX imports with exact-baseline semantic reconciliation.

use crate::{
    DocumentBlockView, DocumentCellView, DocumentRowView, DocumentTableView, ProjectError,
    ProjectService,
};
use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_document::{
    Document, DocumentBlock, DocumentExtension, Section, Table, TableCell, TableRow,
};

impl ProjectService {
    /// Verifies an observed DOCX edit against its persisted bytes and canonical tree.
    ///
    /// External edits that merge to the observed semantic content continue to
    /// the normal project scan/import flow. A merge containing canonical-only
    /// paragraph changes becomes a durable candidate bound to the canonical
    /// source bytes and exact observed filesystem bytes.
    ///
    /// # Parameters
    /// - `project`: Project owning the observed path.
    /// - `path`: Project-relative path reported by the watcher.
    /// - `observed_digest`: Digest reported for the external bytes.
    ///
    /// # Returns
    /// A reconciliation candidate, or `None` when ordinary import is safe.
    ///
    /// # Errors
    /// Returns an error when bytes race the event, baseline/current state is
    /// unavailable, or reconciliation detects conflict or ambiguity.
    pub(crate) fn reconcile_external_docx_observation(
        &self,
        project: ProjectId,
        path: &str,
        observed_digest: [u8; 32],
    ) -> Result<Option<crate::ProjectCandidate>, ProjectError> {
        let prior_files = self
            .store
            .project_files(project)
            .map_err(crate::errors::storage_error)?;
        let Some(baseline) = prior_files.iter().find(|file| file.relative_path == path) else {
            return Ok(None);
        };
        if baseline.adapter != "docx" {
            return Ok(None);
        }
        let live_path = self.workspace(project)?.project.path.join(path);
        crate::project_security::require_public_path(&live_path)?;
        let observed = std::fs::read(&live_path).map_err(|error| ProjectError {
            code: "docx_observation_unavailable",
            message: format!("cannot read observed DOCX source: {error}"),
            diagnostics: Vec::new(),
        })?;
        if blake3::hash(&observed).as_bytes() != &observed_digest {
            return Err(ProjectError {
                code: "docx_observation_raced",
                message: "DOCX bytes changed after the watcher reported its digest".to_owned(),
                diagnostics: Vec::new(),
            });
        }
        let workspace = self.workspace(project)?;
        let document_id = scriberig_domain_docx::semantic_document_id(baseline.entity);
        let current_view = self.document_tree(project, workspace.snapshot, document_id)?;
        let current = document_from_view(
            current_view.document,
            current_view.styles,
            current_view.sections,
        );
        let outcome = scriberig_domain_docx::reconcile_external_docx(
            &baseline.bytes,
            baseline.digest,
            &current,
            &observed,
            document_id,
        );
        match outcome {
            scriberig_domain_docx::DocxReconciliationOutcome::Merged(merged) => {
                let external = scriberig_domain_docx::import_docx(&observed, document_id)
                    .map_err(|message| reconciliation_error("docx_external_parse_failed", message))?
                    .document;
                if merged == external {
                    return Ok(None);
                }
                let merged_bytes =
                    scriberig_domain_docx::materialize_reconciled_docx(&observed, &merged)
                        .map_err(|message| {
                            reconciliation_error("docx_merge_materialization_unsupported", message)
                        })?;
                let candidate = self.propose_external_docx_reconciliation(
                    project,
                    workspace.snapshot,
                    path.to_owned(),
                    observed,
                    merged_bytes,
                    document_id,
                )?;
                Ok(Some(candidate))
            }
            scriberig_domain_docx::DocxReconciliationOutcome::Conflict(paragraphs) => {
                Err(reconciliation_error(
                    "docx_reconciliation_conflict",
                    format!("external and canonical edits conflict at paragraphs {paragraphs:?}"),
                ))
            }
            scriberig_domain_docx::DocxReconciliationOutcome::Ambiguous(reason) => Err(
                reconciliation_error("docx_reconciliation_ambiguous", reason),
            ),
        }
    }
}

/// Reconstructs the typed document model from a bounded snapshot view.
///
/// # Parameters
/// - `id`: Stable document identity.
/// - `styles`: Named style records returned by the project snapshot.
/// - `sections`: Ordered semantic sections from the snapshot.
///
/// # Returns
/// Domain document value used by the DOCX reconciler.
fn document_from_view(
    id: EntityId,
    styles: Vec<crate::DocumentStyleView>,
    sections: Vec<crate::DocumentSectionView>,
) -> Document {
    Document {
        id,
        sections: sections
            .into_iter()
            .map(|section| Section {
                id: section.id,
                blocks: section.blocks.into_iter().map(block_from_view).collect(),
            })
            .collect(),
        lists: Vec::new(),
        drawings: Vec::new(),
        extensions: styles
            .into_iter()
            .map(|style| {
                DocumentExtension::Style(scriberig_domain_document::DocumentStyle {
                    key: style.key,
                    name: style.name,
                })
            })
            .collect(),
    }
}

/// Converts one project block view into a domain block.
///
/// # Parameters
/// - `block`: Typed block returned from the exact snapshot.
///
/// # Returns
/// Equivalent domain block retaining semantic identities.
fn block_from_view(block: DocumentBlockView) -> DocumentBlock {
    match block {
        DocumentBlockView::Paragraph { paragraph } => DocumentBlock::Paragraph(paragraph),
        DocumentBlockView::Table { table } => DocumentBlock::Table(table_from_view(table)),
    }
}

/// Converts a project table view while retaining row and cell identities.
///
/// # Parameters
/// - `table`: Ordered table view from the exact snapshot.
///
/// # Returns
/// Equivalent domain table.
fn table_from_view(table: DocumentTableView) -> Table {
    Table {
        id: table.id,
        rows: table.rows.into_iter().map(row_from_view).collect(),
    }
}

/// Converts a project row view into a domain row.
///
/// # Parameters
/// - `row`: Ordered row view from the exact snapshot.
///
/// # Returns
/// Equivalent domain row.
fn row_from_view(row: DocumentRowView) -> TableRow {
    TableRow {
        id: row.id,
        cells: row.cells.into_iter().map(cell_from_view).collect(),
    }
}

/// Converts a project cell view into a domain cell.
///
/// # Parameters
/// - `cell`: Ordered cell view from the exact snapshot.
///
/// # Returns
/// Equivalent domain cell.
fn cell_from_view(cell: DocumentCellView) -> TableCell {
    TableCell {
        id: cell.id,
        blocks: cell.blocks.into_iter().map(block_from_view).collect(),
    }
}

/// Creates a structured DOCX reconciliation error.
///
/// # Parameters
/// - `code`: Stable machine-readable failure code.
/// - `message`: Human-readable conflict or ambiguity detail.
///
/// # Returns
/// Project error that prevents destructive import.
fn reconciliation_error(code: &'static str, message: impl Into<String>) -> ProjectError {
    ProjectError {
        code,
        message: message.into(),
        diagnostics: Vec::new(),
    }
}
