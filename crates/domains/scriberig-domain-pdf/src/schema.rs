//! Defines the PDF domain vocabulary and graph-facing names.

/// Stable PDF domain entity type names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfEntityType {
    PdfDocument,
    PdfPage,
    TextRegion,
    Paragraph,
    Table,
    TableRow,
    TableCell,
    Figure,
    Annotation,
}
impl PdfEntityType {
    #[must_use]
    /// Returns the stable semantic type name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PdfDocument => "PDFDocument",
            Self::PdfPage => "PDFPage",
            Self::TextRegion => "TextRegion",
            Self::Paragraph => "Paragraph",
            Self::Table => "Table",
            Self::TableRow => "TableRow",
            Self::TableCell => "TableCell",
            Self::Figure => "Figure",
            Self::Annotation => "Annotation",
        }
    }
}
