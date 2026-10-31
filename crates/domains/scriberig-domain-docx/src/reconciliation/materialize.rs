//! Applies a reconciled DOCX semantic result to the exact external package.

use scriberig_domain_document::{Document, Paragraph};

use super::same_structure;

/// Materializes supported canonical-only paragraph changes into external bytes.
///
/// The external package is the preservation source, so its opaque members and
/// external-only changes remain intact. Only simple one-run paragraph text
/// changes can be applied; structural, formatting, and run-shape changes fail
/// closed. The output is reopened and compared with the requested merge.
///
/// # Parameters
/// - `external_bytes`: Exact observed package bytes.
/// - `merged`: Complete semantic result of three-way reconciliation.
///
/// # Returns
/// A reopened DOCX package containing the merged semantic document.
///
/// # Errors
/// Returns an error for unsupported semantic differences, unsafe paragraph
/// rewrites, malformed packages, or output that does not match the merge.
pub fn materialize_reconciled_docx(
    external_bytes: &[u8],
    merged: &Document,
) -> Result<Vec<u8>, String> {
    let external = crate::import_docx(external_bytes, merged.id)?.document;
    if !same_structure(&external, merged) || external.extensions != merged.extensions {
        return Err(
            "merged DOCX structure or extensions differ from the external package".to_owned(),
        );
    }
    let external_paragraphs = collect_paragraphs(&external);
    let merged_paragraphs = collect_paragraphs(merged);
    let document_id = merged.id;
    if external_paragraphs.len() != merged_paragraphs.len() {
        return Err("merged DOCX paragraph inventory differs from the external package".to_owned());
    }
    let mut output = external_bytes.to_vec();
    for (external, merged) in external_paragraphs.iter().zip(&merged_paragraphs) {
        if external == merged {
            continue;
        }
        if !text_only_change(external, merged) {
            return Err(format!(
                "paragraph {} has a non-text change unsupported by the DOCX writer",
                external.id
            ));
        }
        output =
            crate::replace_simple_paragraph(&output, document_id, merged.id, &merged.runs[0].text)?;
    }
    let reopened = crate::import_docx(&output, document_id)?.document;
    if reopened != *merged {
        return Err("materialized DOCX semantics do not equal the merged document".to_owned());
    }
    Ok(output)
}

/// Collects ordered top-level and table-cell paragraphs.
///
/// # Parameters
/// - `document`: Semantic document being traversed.
///
/// # Returns
/// Paragraph references in document order.
fn collect_paragraphs(document: &Document) -> Vec<&Paragraph> {
    let mut result = Vec::new();
    for section in &document.sections {
        collect_blocks(&section.blocks, &mut result);
    }
    result
}

/// Collects paragraphs recursively from blocks.
///
/// # Parameters
/// - `blocks`: Ordered blocks to traverse.
/// - `result`: Destination paragraph references.
fn collect_blocks<'a>(
    blocks: &'a [scriberig_domain_document::DocumentBlock],
    result: &mut Vec<&'a Paragraph>,
) {
    for block in blocks {
        match block {
            scriberig_domain_document::DocumentBlock::Paragraph(paragraph) => {
                result.push(paragraph);
            }
            scriberig_domain_document::DocumentBlock::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_blocks(&cell.blocks, result);
                    }
                }
            }
            scriberig_domain_document::DocumentBlock::Drawing(_) => {}
        }
    }
}

/// Restricts a package rewrite to one simple paragraph text replacement.
///
/// # Parameters
/// - `external`: Paragraph parsed from observed package bytes.
/// - `merged`: Corresponding paragraph in the three-way result.
///
/// # Returns
/// True when the one inline text value changed and all other structure is equal.
fn text_only_change(external: &Paragraph, merged: &Paragraph) -> bool {
    external.id == merged.id
        && external.style == merged.style
        && external.runs.len() == 1
        && merged.runs.len() == 1
        && external.runs[0].id == merged.runs[0].id
        && external.runs[0].style == merged.runs[0].style
        && external.runs[0].hyperlink == merged.runs[0].hyperlink
        && external.runs[0].text != merged.runs[0].text
}
