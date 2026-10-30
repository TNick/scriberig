//! Defines format-independent document structure and addressable content.

pub mod registration;
mod schema;

pub use schema::{
    Bookmark, Comment, ContentControl, Document, DocumentBlock, DocumentExtension, DocumentStyle,
    DrawingObject, Field, HeaderFooter, InlineRun, List, Note, Paragraph, Section, Table,
    TableCell, TableRow, TextStyleRef, TrackedChange,
};

#[cfg(test)]
mod tests {
    use scriberig_core::EntityId;

    use super::{
        Document, DocumentBlock, InlineRun, Paragraph, Section, Table, TableCell, TableRow,
    };

    /// Creates a paragraph with one stable inline run.
    ///
    /// # Parameters
    /// - `id`: Stable identity assigned to the paragraph.
    /// - `text`: Authored text stored in the paragraph's inline run.
    ///
    /// # Returns
    /// A paragraph containing one new stable inline run.
    fn paragraph(id: EntityId, text: &str) -> Paragraph {
        Paragraph {
            id,
            runs: vec![InlineRun {
                id: EntityId::new(),
                text: text.to_owned(),
                style: None,
                hyperlink: None,
            }],
            style: None,
        }
    }

    /// Keeps table cell identity independent from unrelated paragraph changes.
    #[test]
    fn document_graph_addresses_paragraphs_and_table_cells() {
        let cell_id = EntityId::new();
        let document = Document {
            id: EntityId::new(),
            sections: vec![Section {
                id: EntityId::new(),
                blocks: vec![
                    DocumentBlock::Paragraph(paragraph(EntityId::new(), "Existing site area")),
                    DocumentBlock::Paragraph(paragraph(EntityId::new(), "Preserve this paragraph")),
                    DocumentBlock::Table(Table {
                        id: EntityId::new(),
                        rows: vec![TableRow {
                            id: EntityId::new(),
                            cells: vec![TableCell {
                                id: cell_id,
                                blocks: vec![DocumentBlock::Paragraph(paragraph(
                                    EntityId::new(),
                                    "parcel 124",
                                ))],
                            }],
                        }],
                    }),
                ],
            }],
            lists: Vec::new(),
            drawings: Vec::new(),
            extensions: Vec::new(),
        };
        let DocumentBlock::Table(table) = &document.sections[0].blocks[2] else {
            panic!("expected table block");
        };
        assert_eq!(table.rows[0].cells[0].id, cell_id);
        assert_eq!(document.sections[0].blocks.len(), 3);
    }
}
