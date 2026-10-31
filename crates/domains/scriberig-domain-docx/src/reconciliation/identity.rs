//! Resolves paragraph identity evidence for DOCX reconciliation.

use scriberig_core::EntityId;
use scriberig_domain_document::{Document, DocumentBlock, Paragraph};

/// Identifies persistent paragraph entities reordered by the external document.
///
/// # Parameters
/// - `baseline`: Prior imported document.
/// - `external`: Current imported external document.
/// - `stable_ids`: Paragraphs carrying persistent source identities.
///
/// # Returns
/// Stable paragraph identities whose document order changed, or an empty list
/// when identity evidence is incomplete or the change is not a pure reorder.
pub(super) fn stable_paragraph_order_conflicts(
    baseline: &Document,
    external: &Document,
    stable_ids: &std::collections::BTreeSet<EntityId>,
) -> Vec<EntityId> {
    let baseline_order = collect_paragraphs(baseline)
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| stable_ids.contains(id))
        .collect::<Vec<_>>();
    let external_order = collect_paragraphs(external)
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| stable_ids.contains(id))
        .collect::<Vec<_>>();
    if baseline_order.len() != external_order.len()
        || baseline_order
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            != external_order
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
        || baseline_order == external_order
    {
        return Vec::new();
    }
    let mut conflicts = baseline_order
        .iter()
        .zip(&external_order)
        .filter_map(|(before, after)| (before != after).then_some(*before))
        .collect::<Vec<_>>();
    conflicts.sort();
    conflicts.dedup();
    conflicts
}

/// Rejects fallback positional changes without independent semantic anchors.
///
/// A persistent `w14:paraId` is direct identity evidence. For older DOCX files
/// without it, one changed paragraph can retain identity only when at least one
/// other paragraph remains semantically unchanged. Multiple changed positional
/// paragraphs are ambiguous because moves, splits, or merges can resemble edits.
///
/// # Parameters
/// - `baseline`: Exact imported semantic baseline.
/// - `external`: Imported external semantic document.
/// - `stable_ids`: Paragraph identities backed by source `w14:paraId` values.
///
/// # Returns
/// True when positional identity changes cannot be safely distinguished.
pub(super) fn fallback_identity_is_ambiguous(
    baseline: &Document,
    external: &Document,
    stable_ids: &std::collections::BTreeSet<EntityId>,
) -> bool {
    let baseline_paragraphs = collect_paragraphs(baseline);
    let external_paragraphs = collect_paragraphs(external);
    let changed_positional = baseline_paragraphs
        .iter()
        .zip(&external_paragraphs)
        .filter(|((base_id, base), (external_id, observed))| {
            base_id == external_id
                && !stable_ids.contains(base_id)
                && !same_paragraph_content(base, observed)
        })
        .count();
    if changed_positional > 1 {
        return true;
    }
    changed_positional == 1
        && !baseline_paragraphs.iter().zip(&external_paragraphs).any(
            |((base_id, base), (external_id, observed))| {
                base_id == external_id
                    && (stable_ids.contains(base_id) || same_paragraph_content(base, observed))
            },
        )
}

/// Detects unchanged paragraph content appearing under a different positional ID.
///
/// # Parameters
/// - `baseline`: Exact imported baseline document.
/// - `external`: Imported external observation.
///
/// # Returns
/// True when a baseline paragraph's content appears at another identity.
pub(super) fn external_reuses_other_baseline_identity(
    baseline: &Document,
    external: &Document,
) -> bool {
    let baseline_paragraphs = collect_paragraphs(baseline);
    let external_paragraphs = collect_paragraphs(external);
    external_paragraphs.iter().any(|(external_id, paragraph)| {
        baseline_paragraphs.iter().any(|(base_id, base)| {
            base_id != external_id && same_paragraph_content(base, paragraph)
        })
    })
}

/// Collects every paragraph identity and value, including table cells.
///
/// # Parameters
/// - `document`: Semantic document to inspect.
///
/// # Returns
/// Paragraphs in document order, paired with their stable identities.
fn collect_paragraphs(document: &Document) -> Vec<(EntityId, Paragraph)> {
    let mut paragraphs = Vec::new();
    for section in &document.sections {
        collect_paragraphs_from_blocks(&section.blocks, &mut paragraphs);
    }
    paragraphs
}

/// Collects paragraphs recursively from document blocks.
///
/// # Parameters
/// - `blocks`: Blocks being traversed.
/// - `paragraphs`: Destination for paragraph identity and value pairs.
fn collect_paragraphs_from_blocks(
    blocks: &[DocumentBlock],
    paragraphs: &mut Vec<(EntityId, Paragraph)>,
) {
    for block in blocks {
        match block {
            DocumentBlock::Paragraph(paragraph) => {
                paragraphs.push((paragraph.id, paragraph.clone()));
            }
            DocumentBlock::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_paragraphs_from_blocks(&cell.blocks, paragraphs);
                    }
                }
            }
            DocumentBlock::Drawing(_) => {}
        }
    }
}

/// Compares paragraph semantics without considering positional entity IDs.
///
/// # Parameters
/// - `left`: First paragraph value.
/// - `right`: Second paragraph value.
///
/// # Returns
/// True when authored style and inline values match exactly.
fn same_paragraph_content(left: &Paragraph, right: &Paragraph) -> bool {
    left.style == right.style
        && left.runs.len() == right.runs.len()
        && left
            .runs
            .iter()
            .zip(&right.runs)
            .all(|(a, b)| a.text == b.text && a.style == b.style && a.hyperlink == b.hyperlink)
}
