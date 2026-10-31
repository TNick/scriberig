//! Resolves stable paragraph and table-cell handles in imported DOCX semantics.

use scriberig_core::EntityId;
use scriberig_domain_document::{Document, DocumentBlock, Paragraph, TableCell};

use crate::{ProjectError, docx_candidate::docx_candidate_error};

/// Finds the containing table cell for one paragraph in an imported document.
///
/// # Parameters
/// - `document`: Parsed semantic document tree.
/// - `paragraph`: Stable paragraph identity to locate.
///
/// # Returns
/// Stable table-cell identity when the paragraph is inside a cell.
pub(crate) fn table_cell_containing_paragraph(
    document: &Document,
    paragraph: EntityId,
) -> Option<EntityId> {
    document
        .sections
        .iter()
        .find_map(|section| find_cell_in_blocks(&section.blocks, paragraph, None))
}

/// Finds one table cell recursively by its stable semantic identity.
///
/// # Parameters
/// - `document`: Parsed semantic document tree.
/// - `cell`: Stable table-cell identity to resolve.
///
/// # Returns
/// Matching cell reference when it exists.
pub(crate) fn find_table_cell(document: &Document, cell: EntityId) -> Option<&TableCell> {
    /// Searches nested tables for the exact stable cell identity.
    ///
    /// # Parameters
    /// - `blocks`: Semantic blocks containing zero or more tables.
    /// - `cell`: Stable cell identity to locate.
    ///
    /// # Returns
    /// Matching cell reference when it exists.
    fn find_in_blocks(blocks: &[DocumentBlock], cell: EntityId) -> Option<&TableCell> {
        for block in blocks {
            let DocumentBlock::Table(table) = block else {
                continue;
            };
            for row in &table.rows {
                for value in &row.cells {
                    if value.id == cell {
                        return Some(value);
                    }
                    if let Some(found) = find_in_blocks(&value.blocks, cell) {
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
        .find_map(|section| find_in_blocks(&section.blocks, cell))
}

/// Resolves the single editable paragraph in a simple DOCX table cell.
///
/// # Parameters
/// - `cell`: Semantic table cell to edit.
///
/// # Returns
/// The sole paragraph in the cell.
///
/// # Errors
/// Returns a structured edit error for empty, multi-block, nested-table, or
/// non-paragraph cell content.
pub(crate) fn single_cell_paragraph(cell: &TableCell) -> Result<&Paragraph, ProjectError> {
    if let [DocumentBlock::Paragraph(paragraph)] = cell.blocks.as_slice() {
        return Ok(paragraph);
    }
    Err(docx_candidate_error(
        "docx_cell_structure_unsupported",
        "cell editing currently requires exactly one paragraph and no nested table",
    ))
}

/// Traverses nested document blocks while retaining the current table cell.
///
/// # Parameters
/// - `blocks`: Ordered semantic blocks to inspect.
/// - `paragraph`: Stable paragraph identity to locate.
/// - `cell`: Current containing cell, if traversal is inside a table.
///
/// # Returns
/// The cell identity containing the paragraph, if any.
fn find_cell_in_blocks(
    blocks: &[DocumentBlock],
    paragraph: EntityId,
    cell: Option<EntityId>,
) -> Option<EntityId> {
    for block in blocks {
        match block {
            DocumentBlock::Paragraph(value) if value.id == paragraph => return cell,
            DocumentBlock::Table(table) => {
                for row in &table.rows {
                    for child in &row.cells {
                        if let Some(found) =
                            find_cell_in_blocks(&child.blocks, paragraph, Some(child.id))
                        {
                            return Some(found);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}
