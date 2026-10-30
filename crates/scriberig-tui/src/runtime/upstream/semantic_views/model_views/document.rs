//! Renders bounded semantic document outlines and paragraph/table content.

use super::super::common::{append_value, render_load_state};
use crate::{inspector::LoadState, selection::EntitySelection};
use scriberig_protocol::{DocumentBlockView, DocumentInspectResponse};

/// Maximum document records rendered in one terminal projection.
const MAX_DOCUMENT_ROWS: usize = 80;
/// Maximum nested table recursion rendered in one terminal projection.
const MAX_DOCUMENT_DEPTH: usize = 16;
/// Maximum visible authored text emitted for one paragraph/cell summary.
const MAX_PARAGRAPH_CHARS: usize = 240;

/// Renders section, paragraph, and table summaries from an exact snapshot.
///
/// # Parameters
/// - `state`: Document tree request state.
/// - `selection`: Current exact semantic selection, when present.
/// - `offset`: First row index to include in the bounded document view.
///
/// # Returns
/// Bounded Markdown for the loaded document or a safe load-state message.
pub(super) fn render(
    state: &LoadState<DocumentInspectResponse>,
    selection: Option<EntitySelection>,
    offset: usize,
) -> String {
    render_load_state("Semantic document", state, |tree, output| {
        append_value(output, &format!("Document: {}", tree.document));
        append_value(output, &format!("Snapshot: {}", tree.snapshot));
        if let Some(capability) = tree.artifact_capability {
            append_value(output, &format!("Edit capability: {capability:?}"));
        }
        let selected = selection.map(|value| value.entity.to_string());
        let mut rows = Vec::new();
        for section in &tree.sections {
            rows.push(DocumentRow {
                entity: section.id.clone(),
                label: format!("Section {}", section.id),
            });
            collect_blocks(&section.blocks, &mut rows, 0);
        }
        let visible = rows
            .iter()
            .skip(offset)
            .take(MAX_DOCUMENT_ROWS)
            .collect::<Vec<_>>();
        if visible.is_empty() {
            append_value(
                output,
                &format!(
                    "No document rows at offset {offset}; the outline has {} rows.",
                    rows.len()
                ),
            );
            return;
        }
        for row in &visible {
            let marker = if selected
                .as_ref()
                .is_some_and(|entity| entity == &row.entity)
            {
                "› "
            } else {
                "  "
            };
            append_value(output, &format!("{marker}{}", row.label));
        }
        if offset > 0 || offset.saturating_add(visible.len()) < rows.len() {
            append_value(
                output,
                &format!(
                    "Rows {}–{} of {}. Use /document <offset> to page.",
                    offset.saturating_add(1),
                    offset.saturating_add(visible.len()),
                    rows.len()
                ),
            );
        }
    })
}

/// One addressable semantic line in the flattened document outline.
struct DocumentRow {
    /// Stable semantic entity associated with the line.
    entity: String,
    /// Bounded text shown in the terminal projection.
    label: String,
}

/// Flattens nested paragraphs and tables into ordered, addressable outline rows.
///
/// # Parameters
/// - `blocks`: Ordered semantic blocks to render.
/// - `rows`: Output rows receiving semantic identities and summaries.
/// - `depth`: Current nested table depth.
fn collect_blocks(blocks: &[DocumentBlockView], rows: &mut Vec<DocumentRow>, depth: usize) {
    if depth >= MAX_DOCUMENT_DEPTH {
        rows.push(DocumentRow {
            entity: String::new(),
            label: "Nested content omitted beyond the supported view depth.".to_owned(),
        });
        return;
    }
    for block in blocks {
        match block {
            DocumentBlockView::Paragraph { paragraph } => {
                let text = paragraph
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>();
                rows.push(DocumentRow {
                    entity: paragraph.id.clone(),
                    label: format!(
                        "Paragraph {} · {}",
                        paragraph.id,
                        truncate(&text, MAX_PARAGRAPH_CHARS)
                    ),
                });
            }
            DocumentBlockView::Table { table } => {
                rows.push(DocumentRow {
                    entity: table.id.clone(),
                    label: format!("Table {}", table.id),
                });
                for row in &table.rows {
                    rows.push(DocumentRow {
                        entity: row.id.clone(),
                        label: format!("  Row {}", row.id),
                    });
                    for cell in &row.cells {
                        rows.push(DocumentRow {
                            entity: cell.id.clone(),
                            label: format!("    Cell {}", cell.id),
                        });
                        collect_blocks(&cell.blocks, rows, depth + 1);
                    }
                }
            }
        }
    }
}

/// Bounds long authored text while retaining a visible omission marker.
///
/// # Parameters
/// - `text`: Authored paragraph or cell text.
/// - `limit`: Maximum visible Unicode scalar count.
///
/// # Returns
/// Text no longer than the requested bound, with an ellipsis when shortened.
fn truncate(text: &str, limit: usize) -> String {
    let mut characters = text.chars();
    let visible = characters.by_ref().take(limit).collect::<String>();
    if characters.next().is_some() {
        format!("{visible}…")
    } else {
        visible
    }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::inspector::LoadState;
    use scriberig_protocol::{
        DocumentBlockView, DocumentInlineRunView, DocumentInspectResponse, DocumentParagraphView,
        DocumentSectionView,
    };

    /// Renders stable paragraph and table-cell identities with bounded authored content.
    #[test]
    fn overview_renders_document_outline_and_semantic_text() {
        let tree = DocumentInspectResponse {
            project: "project-1".to_owned(),
            snapshot: "snapshot-1".to_owned(),
            document: "document-1".to_owned(),
            artifact_capability: None,
            styles: Vec::new(),
            sections: vec![DocumentSectionView {
                id: "section-1".to_owned(),
                blocks: vec![DocumentBlockView::Paragraph {
                    paragraph: DocumentParagraphView {
                        id: "paragraph-1".to_owned(),
                        runs: vec![DocumentInlineRunView {
                            id: "run-1".to_owned(),
                            text: "Opening paragraph".to_owned(),
                            style: None,
                            hyperlink: None,
                        }],
                        style: None,
                    },
                }],
            }],
        };
        let output = render(&LoadState::Ready(tree.clone()), None, 0);
        assert!(output.contains("Section section-1"));
        assert!(output.contains("Paragraph paragraph-1 · Opening paragraph"));
        let next_page = render(&LoadState::Ready(tree), None, 1);
        assert!(!next_page.contains("Section section-1"));
        assert!(next_page.contains("Paragraph paragraph-1 · Opening paragraph"));
    }
}
