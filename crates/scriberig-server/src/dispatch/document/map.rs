//! Maps snapshot-bound project document views into protocol DTOs.

use scriberig_protocol::{
    DocumentBlockView as WireBlock, DocumentCellView as WireCell, DocumentInlineRunView,
    DocumentParagraphView, DocumentRowView as WireRow, DocumentSectionView as WireSection,
    DocumentTableView as WireTable, DocumentTextStyleRef,
};

/// Converts one project section into the stable protocol DTO.
///
/// # Parameters
/// - `section`: Project section with ordered semantic blocks.
///
/// # Returns
/// Stable protocol section DTO.
pub(super) fn map_section(section: scriberig_app::DocumentSectionView) -> WireSection {
    WireSection {
        id: section.id.to_string(),
        blocks: section.blocks.into_iter().map(map_block).collect(),
    }
}

/// Converts one project semantic block into the stable protocol DTO.
///
/// # Parameters
/// - `block`: Typed project paragraph or table block.
///
/// # Returns
/// Stable protocol block DTO.
fn map_block(block: scriberig_app::DocumentBlockView) -> WireBlock {
    match block {
        scriberig_app::DocumentBlockView::Paragraph { paragraph } => WireBlock::Paragraph {
            paragraph: DocumentParagraphView {
                id: paragraph.id.to_string(),
                runs: paragraph
                    .runs
                    .into_iter()
                    .map(|run| DocumentInlineRunView {
                        id: run.id.to_string(),
                        text: run.text,
                        style: run.style.map(|style| DocumentTextStyleRef {
                            key: style.key,
                            adapter: style.adapter,
                        }),
                        hyperlink: run.hyperlink,
                    })
                    .collect(),
                style: paragraph.style.map(|style| DocumentTextStyleRef {
                    key: style.key,
                    adapter: style.adapter,
                }),
            },
        },
        scriberig_app::DocumentBlockView::Table { table } => WireBlock::Table {
            table: map_table(table),
        },
    }
}

/// Converts one nested project table into the stable protocol DTO.
///
/// # Parameters
/// - `table`: Project table with ordered rows and cells.
///
/// # Returns
/// Stable protocol table DTO.
fn map_table(table: scriberig_app::DocumentTableView) -> WireTable {
    WireTable {
        id: table.id.to_string(),
        rows: table
            .rows
            .into_iter()
            .map(|row| WireRow {
                id: row.id.to_string(),
                cells: row
                    .cells
                    .into_iter()
                    .map(|cell| WireCell {
                        id: cell.id.to_string(),
                        blocks: cell.blocks.into_iter().map(map_block).collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}
