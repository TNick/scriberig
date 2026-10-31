//! Validates staged DOCX packages in isolated candidate workspaces.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use scriberig_core::EntityId;
use scriberig_domain_document::{Document, DocumentBlock, Paragraph, TableCell};
use scriberig_store_sqlite::ProjectFileRecord;

use super::{ProcessFailure, safe_join};

/// Reopens changed DOCX packages from one exact isolated candidate workspace.
///
/// # Parameters
/// - `root`: Isolated candidate filesystem root.
/// - `files`: Exact project source manifest for the candidate base.
/// - `effects`: Exact project source effects staged for validation.
/// - `semantic_operation`: Candidate operation when it identifies document semantics.
/// - `semantic_target`: Exact semantic entity targeted by the candidate.
/// - `cancelled`: Shared cancellation state for the validator.
///
/// # Returns
/// Empty diagnostics on success or bounded package failures.
///
/// # Errors
/// Returns cancellation or isolated file-access failures.
pub(super) fn validate_docx_packages(
    root: &Path,
    files: &[ProjectFileRecord],
    effects: &[super::super::ProjectSourceEffect],
    semantic_operation: Option<&str>,
    semantic_target: Option<EntityId>,
    cancelled: &AtomicBool,
) -> Result<Vec<String>, ProcessFailure> {
    let paths = effects
        .iter()
        .filter(|effect| {
            Path::new(&effect.path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("docx"))
        })
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Ok(vec!["candidate has no DOCX package to validate".into()]);
    }
    let mut diagnostics = Vec::new();
    for effect in paths {
        if cancelled.load(Ordering::Acquire) {
            return Err(ProcessFailure::Cancelled);
        }
        let path =
            safe_join(root, &effect.path).map_err(|error| ProcessFailure::Failed(error.message))?;
        let bytes = fs::read(path).map_err(|error| ProcessFailure::Failed(error.to_string()))?;
        if let Err(error) = scriberig_domain_docx::validate_package(&bytes) {
            diagnostics.push(format!("{} failed validation: {error}", effect.path));
            continue;
        }
        if let Some(operation) = semantic_operation.filter(|operation| {
            matches!(
                *operation,
                "document.replace_paragraph"
                    | "document.insert_paragraph"
                    | "document.update_table_cell"
            )
        }) {
            if let Err(error) =
                validate_semantic_effect(files, effect, &bytes, operation, semantic_target)
            {
                diagnostics.push(error);
            }
        }
    }
    Ok(diagnostics)
}

/// Validates the candidate document against the exact prior semantic tree.
///
/// # Parameters
/// - `files`: Source manifest captured at the exact candidate base.
/// - `effect`: DOCX package change being validated.
/// - `candidate_bytes`: Staged candidate DOCX bytes after package validation.
/// - `operation`: Semantic document operation that produced the candidate.
/// - `target`: Exact paragraph, section, or table-cell identity selected by the operation.
///
/// # Returns
/// Unit when the operation produced only its declared semantic change.
///
/// # Errors
/// Returns a diagnostic mapped to the target semantic handle when tree parsing or
/// operation-specific verification fails.
fn validate_semantic_effect(
    files: &[ProjectFileRecord],
    effect: &super::super::ProjectSourceEffect,
    candidate_bytes: &[u8],
    operation: &str,
    target: Option<EntityId>,
) -> Result<(), String> {
    let handle_kind = match operation {
        "document.replace_paragraph" => "document.paragraph",
        "document.update_table_cell" => "document.table_cell",
        _ => "document.section",
    };
    let target = target.ok_or_else(|| {
        format!("{handle_kind} target identity was not retained by the candidate")
    })?;
    let source = files
        .iter()
        .find(|file| file.relative_path == effect.path && file.adapter == "docx")
        .ok_or_else(|| {
            format!("{handle_kind} {target} has no exact-base DOCX source manifest entry")
        })?;
    let original_bytes = effect
        .old
        .as_deref()
        .ok_or_else(|| format!("{handle_kind} {target} has no exact prior package bytes"))?;
    let document_id = scriberig_domain_docx::semantic_document_id(source.entity);
    let original =
        scriberig_domain_docx::import_docx(original_bytes, document_id).map_err(|_| {
            format!("{handle_kind} {target} could not be resolved in the exact prior DOCX")
        })?;
    let candidate =
        scriberig_domain_docx::import_docx(candidate_bytes, document_id).map_err(|_| {
            format!("{handle_kind} {target} could not be resolved in the candidate DOCX")
        })?;
    verify_document_operation(&original.document, candidate.document, operation, target)
}

/// Verifies a paragraph, table-cell, or section operation against two semantic trees.
///
/// # Parameters
/// - `original`: Exact prior document semantic tree.
/// - `candidate`: Reopened candidate semantic tree.
/// - `operation`: Stable semantic operation name.
/// - `target`: Exact paragraph, table-cell, or section identity targeted by the operation.
///
/// # Returns
/// Unit when the target change and all unrelated semantic content match.
///
/// # Errors
/// Returns a target-mapped diagnostic when the target is missing or unrelated
/// semantic content changed.
fn verify_document_operation(
    original: &Document,
    mut candidate: Document,
    operation: &str,
    target: EntityId,
) -> Result<(), String> {
    if operation == "document.replace_paragraph" {
        let previous = find_paragraph(original, target).cloned().ok_or_else(|| {
            format!("document.paragraph {target} is missing from the exact prior document")
        })?;
        let updated = find_paragraph(&candidate, target).ok_or_else(|| {
            format!("document.paragraph {target} is missing from the candidate document")
        })?;
        if *updated == previous {
            return Err(format!(
                "document.paragraph {target} has no semantic replacement"
            ));
        }
        if !restore_paragraph(&mut candidate, target, &previous) || &candidate != original {
            return Err(format!(
                "document.paragraph {target} changed with unrelated document content"
            ));
        }
        return Ok(());
    }
    if operation == "document.update_table_cell" {
        let previous = find_table_cell(original, target).cloned().ok_or_else(|| {
            format!("document.table_cell {target} is missing from the exact prior document")
        })?;
        let updated = find_table_cell(&candidate, target).ok_or_else(|| {
            format!("document.table_cell {target} is missing from the candidate document")
        })?;
        if *updated == previous {
            return Err(format!(
                "document.table_cell {target} has no semantic replacement"
            ));
        }
        if !restore_table_cell(&mut candidate, target, &previous) || &candidate != original {
            return Err(format!(
                "document.table_cell {target} changed with unrelated document content"
            ));
        }
        return Ok(());
    }
    if operation == "document.insert_paragraph" {
        let appended = remove_appended_paragraph(&mut candidate, target).ok_or_else(|| {
            format!("document.section {target} has no appended paragraph at its tail")
        })?;
        if appended.runs.is_empty() || &candidate != original {
            return Err(format!(
                "document.section {target} changed with unrelated document content"
            ));
        }
        return Ok(());
    }
    Ok(())
}

/// Finds one table cell recursively by its semantic identity.
///
/// # Parameters
/// - `document`: Document semantic tree to inspect.
/// - `target`: Table-cell identity to resolve.
///
/// # Returns
/// Matching cell reference when present.
fn find_table_cell(document: &Document, target: EntityId) -> Option<&TableCell> {
    /// Finds a target table cell in nested document blocks.
    ///
    /// # Parameters
    /// - `blocks`: Blocks to search, including nested table cells.
    /// - `target`: Table-cell identity to resolve.
    ///
    /// # Returns
    /// Matching cell reference when present.
    fn find_in_blocks(blocks: &[DocumentBlock], target: EntityId) -> Option<&TableCell> {
        for block in blocks {
            if let DocumentBlock::Table(table) = block {
                for cell in table.rows.iter().flat_map(|row| &row.cells) {
                    if cell.id == target {
                        return Some(cell);
                    }
                    if let Some(found) = find_in_blocks(&cell.blocks, target) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }
    document
        .sections
        .iter()
        .find_map(|section| find_in_blocks(&section.blocks, target))
}

/// Restores one prior table cell before comparing complete semantic trees.
///
/// # Parameters
/// - `document`: Candidate tree to normalize.
/// - `target`: Table-cell identity to restore.
/// - `previous`: Exact prior cell value.
///
/// # Returns
/// Whether the target cell was found and restored.
fn restore_table_cell(document: &mut Document, target: EntityId, previous: &TableCell) -> bool {
    /// Restores a target table cell in nested document blocks.
    ///
    /// # Parameters
    /// - `blocks`: Mutable blocks to search, including nested table cells.
    /// - `target`: Table-cell identity to restore.
    /// - `previous`: Exact prior cell value.
    ///
    /// # Returns
    /// Whether the target cell was found and restored.
    fn restore_in_blocks(
        blocks: &mut [DocumentBlock],
        target: EntityId,
        previous: &TableCell,
    ) -> bool {
        for block in blocks {
            if let DocumentBlock::Table(table) = block {
                for cell in table.rows.iter_mut().flat_map(|row| &mut row.cells) {
                    if cell.id == target {
                        *cell = previous.clone();
                        return true;
                    }
                    if restore_in_blocks(&mut cell.blocks, target, previous) {
                        return true;
                    }
                }
            }
        }
        false
    }
    document
        .sections
        .iter_mut()
        .any(|section| restore_in_blocks(&mut section.blocks, target, previous))
}

/// Finds one paragraph recursively by its semantic identity.
///
/// # Parameters
/// - `document`: Document semantic tree to inspect.
/// - `target`: Paragraph identity to resolve.
///
/// # Returns
/// Matching paragraph reference when present.
fn find_paragraph(document: &Document, target: EntityId) -> Option<&Paragraph> {
    /// Finds a target paragraph in nested document blocks.
    ///
    /// # Parameters
    /// - `blocks`: Blocks to search, including nested table cells.
    /// - `target`: Paragraph identity to resolve.
    ///
    /// # Returns
    /// Matching paragraph reference when present.
    fn find_in_blocks(blocks: &[DocumentBlock], target: EntityId) -> Option<&Paragraph> {
        for block in blocks {
            match block {
                DocumentBlock::Paragraph(paragraph) if paragraph.id == target => {
                    return Some(paragraph);
                }
                DocumentBlock::Table(table) => {
                    for cell in table.rows.iter().flat_map(|row| &row.cells) {
                        if let Some(paragraph) = find_in_blocks(&cell.blocks, target) {
                            return Some(paragraph);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
    document
        .sections
        .iter()
        .find_map(|section| find_in_blocks(&section.blocks, target))
}

/// Restores one prior paragraph in a candidate tree before whole-tree comparison.
///
/// # Parameters
/// - `document`: Candidate semantic tree to normalize.
/// - `target`: Paragraph identity to restore.
/// - `previous`: Exact prior semantic paragraph value.
///
/// # Returns
/// Whether the target paragraph was found and restored.
fn restore_paragraph(document: &mut Document, target: EntityId, previous: &Paragraph) -> bool {
    /// Restores a target paragraph in nested document blocks.
    ///
    /// # Parameters
    /// - `blocks`: Mutable blocks to search, including nested table cells.
    /// - `target`: Paragraph identity to restore.
    /// - `previous`: Exact prior paragraph value.
    ///
    /// # Returns
    /// Whether the target paragraph was found and restored.
    fn restore_in_blocks(
        blocks: &mut [DocumentBlock],
        target: EntityId,
        previous: &Paragraph,
    ) -> bool {
        for block in blocks {
            match block {
                DocumentBlock::Paragraph(paragraph) if paragraph.id == target => {
                    *paragraph = previous.clone();
                    return true;
                }
                DocumentBlock::Table(table) => {
                    for cell in table.rows.iter_mut().flat_map(|row| &mut row.cells) {
                        if restore_in_blocks(&mut cell.blocks, target, previous) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
    document
        .sections
        .iter_mut()
        .any(|section| restore_in_blocks(&mut section.blocks, target, previous))
}

/// Removes one appended tail paragraph from an exact semantic section.
///
/// # Parameters
/// - `document`: Candidate semantic tree to normalize.
/// - `target`: Section identity that received the new paragraph.
///
/// # Returns
/// Removed paragraph when the target section ends in one.
fn remove_appended_paragraph(document: &mut Document, target: EntityId) -> Option<Paragraph> {
    let section = document
        .sections
        .iter_mut()
        .find(|section| section.id == target)?;
    match section.blocks.pop()? {
        DocumentBlock::Paragraph(paragraph) => Some(paragraph),
        block => {
            section.blocks.push(block);
            None
        }
    }
}
