//! Reconciles conservative paragraph edits against an exact DOCX baseline.

use scriberig_core::EntityId;
use scriberig_domain_document::{Document, DocumentBlock, Paragraph, Table};

mod identity;
mod materialize;
use identity::{
    external_reuses_other_baseline_identity, fallback_identity_is_ambiguous,
    stable_paragraph_order_conflicts,
};
pub use materialize::materialize_reconciled_docx;

/// Result of reconciling an externally edited DOCX with canonical state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocxReconciliationOutcome {
    /// External paragraph changes merged without changing structural identity.
    Merged(Document),
    /// Concurrent paragraph content or ordering changes require resolution.
    Conflict(Vec<EntityId>),
    /// Structural identity or baseline evidence was insufficient to merge safely.
    Ambiguous(String),
}

/// Merges external paragraph edits using exact baseline bytes and stable structure.
///
/// Persistent `w14:paraId` values provide paragraph identity independent of
/// source order. Reordered persistent paragraphs return typed conflicts;
/// structural changes without sufficient identity evidence return ambiguity.
/// Older files without persistent IDs require unchanged semantic anchors.
/// Extensions remain from canonical state, so parser omissions never imply
/// deletion of opaque DOCX content.
///
/// # Parameters
/// - `baseline_bytes`: Exact package bytes from the prior synchronization.
/// - `baseline_digest`: Expected digest of those exact bytes.
/// - `current`: Current canonical semantic document.
/// - `external_bytes`: Current observed DOCX package bytes.
/// - `document_id`: Stable document identity used by the importer.
///
/// # Returns
/// A merged document, paragraph conflict identities, or an ambiguity reason.
#[must_use]
pub fn reconcile_external_docx(
    baseline_bytes: &[u8],
    baseline_digest: [u8; 32],
    current: &Document,
    external_bytes: &[u8],
    document_id: EntityId,
) -> DocxReconciliationOutcome {
    if blake3::hash(baseline_bytes).as_bytes() != &baseline_digest {
        return DocxReconciliationOutcome::Ambiguous(
            "exact DOCX synchronization baseline digest does not match".to_owned(),
        );
    }
    let baseline_import = match crate::import_docx(baseline_bytes, document_id) {
        Ok(imported) => imported,
        Err(error) => {
            return DocxReconciliationOutcome::Ambiguous(format!(
                "DOCX synchronization baseline cannot be parsed: {error}"
            ));
        }
    };
    let external_import = match crate::import_docx(external_bytes, document_id) {
        Ok(imported) => imported,
        Err(error) => {
            return DocxReconciliationOutcome::Ambiguous(format!(
                "external DOCX cannot be parsed: {error}"
            ));
        }
    };
    let baseline = baseline_import.document;
    let external = external_import.document;
    if !same_structure(&baseline, current) {
        return DocxReconciliationOutcome::Ambiguous(
            "canonical DOCX structure changed since the synchronization baseline".to_owned(),
        );
    }
    if !same_structure(&baseline, &external) {
        let conflicts = stable_paragraph_order_conflicts(
            &baseline,
            &external,
            &baseline_import.stable_paragraph_ids,
        );
        return if conflicts.is_empty() {
            DocxReconciliationOutcome::Ambiguous(
                "external DOCX structure changed; semantic continuity cannot be proven".to_owned(),
            )
        } else {
            DocxReconciliationOutcome::Conflict(conflicts)
        };
    }
    if external_reuses_other_baseline_identity(&baseline, &external) {
        return DocxReconciliationOutcome::Ambiguous(
            "external paragraph content moved across positional identities".to_owned(),
        );
    }
    if fallback_identity_is_ambiguous(&baseline, &external, &baseline_import.stable_paragraph_ids) {
        return DocxReconciliationOutcome::Ambiguous(
            "changed positional paragraph identities have insufficient semantic anchors".to_owned(),
        );
    }
    let mut merged = current.clone();
    let mut conflicts = Vec::new();
    for (base, live, observed) in paragraph_triplets(&baseline, &merged, &external) {
        if observed != base && live != base && observed != live {
            conflicts.push(base.id);
        } else if observed != base && live == base {
            replace_paragraph(&mut merged, &observed);
        }
    }
    if conflicts.is_empty() {
        DocxReconciliationOutcome::Merged(merged)
    } else {
        conflicts.sort();
        conflicts.dedup();
        DocxReconciliationOutcome::Conflict(conflicts)
    }
}

/// Confirms stable section, block, table, paragraph, and run identities.
///
/// # Parameters
/// - `left`: First semantic document.
/// - `right`: Second semantic document.
///
/// # Returns
/// True when both documents have identical addressable structure.
fn same_structure(left: &Document, right: &Document) -> bool {
    left.id == right.id
        && left.sections.len() == right.sections.len()
        && left.sections.iter().zip(&right.sections).all(|(a, b)| {
            a.id == b.id
                && a.blocks.len() == b.blocks.len()
                && a.blocks
                    .iter()
                    .zip(&b.blocks)
                    .all(|(left, right)| block_structure(left, right))
        })
}

/// Compares recursive block identity while allowing paragraph values to differ.
///
/// # Parameters
/// - `left`: First semantic block.
/// - `right`: Second semantic block.
///
/// # Returns
/// True when structural identity is unchanged.
fn block_structure(left: &DocumentBlock, right: &DocumentBlock) -> bool {
    match (left, right) {
        (DocumentBlock::Paragraph(a), DocumentBlock::Paragraph(b)) => {
            a.id == b.id
                && a.runs.len() == b.runs.len()
                && a.runs.iter().zip(&b.runs).all(|(x, y)| x.id == y.id)
        }
        (DocumentBlock::Table(a), DocumentBlock::Table(b)) => table_structure(a, b),
        (DocumentBlock::Drawing(a), DocumentBlock::Drawing(b)) => a.id == b.id,
        _ => false,
    }
}

/// Compares table, row, cell, and nested block identities.
///
/// # Parameters
/// - `left`: First table.
/// - `right`: Second table.
///
/// # Returns
/// True when table structure and identities match exactly.
fn table_structure(left: &Table, right: &Table) -> bool {
    left.id == right.id
        && left.rows.len() == right.rows.len()
        && left.rows.iter().zip(&right.rows).all(|(a, b)| {
            a.id == b.id
                && a.cells.len() == b.cells.len()
                && a.cells.iter().zip(&b.cells).all(|(x, y)| {
                    x.id == y.id
                        && x.blocks.len() == y.blocks.len()
                        && x.blocks
                            .iter()
                            .zip(&y.blocks)
                            .all(|(left, right)| block_structure(left, right))
                })
        })
}

/// Produces matching paragraph references after structure validation.
///
/// # Parameters
/// - `baseline`: Exact imported baseline document.
/// - `current`: Mutable merged document.
/// - `external`: Imported external observation.
///
/// # Returns
/// Paragraph triplets in stable document order.
fn paragraph_triplets(
    baseline: &Document,
    current: &Document,
    external: &Document,
) -> Vec<(Paragraph, Paragraph, Paragraph)> {
    let mut result = Vec::new();
    for ((base_section, current_section), external_section) in baseline
        .sections
        .iter()
        .zip(&current.sections)
        .zip(&external.sections)
    {
        collect_triplets(
            &base_section.blocks,
            &current_section.blocks,
            &external_section.blocks,
            &mut result,
        );
    }
    result
}

/// Collects paragraph triplets recursively from matching block sequences.
///
/// # Parameters
/// - `baseline`: Baseline blocks.
/// - `current`: Current canonical blocks.
/// - `external`: External blocks.
/// - `result`: Destination for matching paragraph references.
fn collect_triplets(
    baseline: &[DocumentBlock],
    current: &[DocumentBlock],
    external: &[DocumentBlock],
    result: &mut Vec<(Paragraph, Paragraph, Paragraph)>,
) {
    for ((base, live), observed) in baseline.iter().zip(current).zip(external) {
        match (base, live, observed) {
            (
                DocumentBlock::Paragraph(a),
                DocumentBlock::Paragraph(b),
                DocumentBlock::Paragraph(c),
            ) => {
                result.push((a.clone(), b.clone(), c.clone()));
            }
            (DocumentBlock::Table(a), DocumentBlock::Table(b), DocumentBlock::Table(c)) => {
                for ((ar, br), cr) in a.rows.iter().zip(&b.rows).zip(&c.rows) {
                    for ((ac, bc), cc) in ar.cells.iter().zip(&br.cells).zip(&cr.cells) {
                        collect_triplets(&ac.blocks, &bc.blocks, &cc.blocks, result);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Replaces the paragraph with its stable identity in the merged document.
///
/// # Parameters
/// - `document`: Mutable merged document.
/// - `replacement`: External paragraph selected by the three-way merge.
fn replace_paragraph(document: &mut Document, replacement: &Paragraph) {
    for section in &mut document.sections {
        if replace_in_blocks(&mut section.blocks, replacement) {
            return;
        }
    }
}

/// Replaces one paragraph in nested block sequences.
///
/// # Parameters
/// - `blocks`: Mutable semantic blocks to search.
/// - `replacement`: External paragraph selected by the merge.
///
/// # Returns
/// True if the target paragraph was replaced.
fn replace_in_blocks(blocks: &mut [DocumentBlock], replacement: &Paragraph) -> bool {
    for block in blocks {
        match block {
            DocumentBlock::Paragraph(paragraph) if paragraph.id == replacement.id => {
                *paragraph = replacement.clone();
                return true;
            }
            DocumentBlock::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if replace_in_blocks(&mut cell.blocks, replacement) {
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    false
}
