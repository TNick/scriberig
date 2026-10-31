//! Defines addressable document blocks, inline content, and style references.

use scriberig_core::EntityId;
use serde::{Deserialize, Serialize};

/// Represents a structured document with ordered sections and semantic content.
///
/// Pagination, rendered line wrapping, and page coordinates stay adapter-derived
/// presentation facts unless a document format explicitly defines them as authored content.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Stable document semantic identity.
    pub id: EntityId,
    /// Sections in authored order.
    pub sections: Vec<Section>,
    /// Lists with explicit ordered item paragraph identities.
    pub lists: Vec<List>,
    /// Images and other non-text drawing references.
    pub drawings: Vec<DrawingObject>,
    /// Inspectable format-specific structures represented as typed records.
    pub extensions: Vec<DocumentExtension>,
}

/// Represents an ordered section of document content.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Section {
    /// Stable section semantic identity.
    pub id: EntityId,
    /// Paragraphs and tables in authored order.
    pub blocks: Vec<DocumentBlock>,
}

/// Represents one independently addressable document block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DocumentBlock {
    /// Paragraph content.
    Paragraph(Paragraph),
    /// Structured table content.
    Table(Table),
    /// Explicit image or drawing reference.
    Drawing(DrawingObject),
}

/// Represents a paragraph whose inline runs retain semantic identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    /// Stable paragraph semantic identity.
    pub id: EntityId,
    /// Ordered authored text runs.
    pub runs: Vec<InlineRun>,
    /// Optional paragraph style reference.
    pub style: Option<TextStyleRef>,
}

/// Represents a text run and its independent formatting/reference data.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InlineRun {
    /// Stable run semantic identity.
    pub id: EntityId,
    /// Authored text content.
    pub text: String,
    /// Optional run style reference.
    pub style: Option<TextStyleRef>,
    /// Explicit destination for a hyperlink run.
    pub hyperlink: Option<String>,
}

/// Represents a table with ordered rows.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Table {
    /// Stable table semantic identity.
    pub id: EntityId,
    /// Rows in authored order.
    pub rows: Vec<TableRow>,
}

/// Represents an ordered table row.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TableRow {
    /// Stable row semantic identity.
    pub id: EntityId,
    /// Cells in authored order.
    pub cells: Vec<TableCell>,
}

/// Represents an independently addressable table cell.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    /// Stable cell semantic identity.
    pub id: EntityId,
    /// Cell paragraphs or nested tables in authored order.
    pub blocks: Vec<DocumentBlock>,
}

/// Represents an ordered or unordered list and its semantic items.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct List {
    /// Stable list semantic identity.
    pub id: EntityId,
    /// Paragraph identities in authored list order.
    pub items: Vec<EntityId>,
    /// Whether list order is numeric rather than bulleted.
    pub ordered: bool,
}

/// Represents an external or embedded image/drawing target.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DrawingObject {
    /// Stable drawing semantic identity.
    pub id: EntityId,
    /// Explicit package or external resource reference.
    pub target: String,
    /// Relationship identity when supplied by the format adapter.
    pub relationship: Option<String>,
}

/// Represents an inspectable document-specific feature without flattening it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DocumentExtension {
    /// Named document style and its adapter-owned identity.
    Style(DocumentStyle),
    /// Header or footer content reference.
    HeaderFooter(HeaderFooter),
    /// Footnote or endnote content.
    Note(Note),
    /// Named structural bookmark.
    Bookmark(Bookmark),
    /// Authored field instruction and optional displayed result.
    Field(Field),
    /// Comment content anchored to a document object.
    Comment(Comment),
    /// Tracked insertion or deletion content.
    TrackedChange(TrackedChange),
    /// Structured content control metadata.
    ContentControl(ContentControl),
}

/// Represents a named document style.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentStyle {
    /// Adapter-owned style key.
    pub key: String,
    /// Human-readable style name when provided.
    pub name: Option<String>,
}

/// Represents a header or footer part reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HeaderFooter {
    /// Stable semantic identity.
    pub id: EntityId,
    /// True for header content, false for footer content.
    pub header: bool,
    /// Explicit adapter-owned package target.
    pub target: String,
}

/// Represents a footnote or endnote.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// Stable semantic identity.
    pub id: EntityId,
    /// True for a footnote, false for an endnote.
    pub footnote: bool,
    /// Ordered note paragraphs.
    pub paragraphs: Vec<Paragraph>,
}

/// Represents a named range bookmark.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    /// Stable semantic identity.
    pub id: EntityId,
    /// Authored bookmark name.
    pub name: String,
    /// Semantic start and end object identities where resolved.
    pub anchored_objects: Vec<EntityId>,
}

/// Represents a field instruction distinct from its displayed result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Stable semantic identity.
    pub id: EntityId,
    /// Authored field instruction.
    pub instruction: String,
    /// Displayed result text when present.
    pub result: Option<String>,
}

/// Represents a comment and its semantic anchor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    /// Stable semantic identity.
    pub id: EntityId,
    /// Comment author when supplied.
    pub author: Option<String>,
    /// Authored comment text.
    pub text: String,
    /// Anchored semantic object identities when resolved.
    pub anchored_objects: Vec<EntityId>,
}

/// Represents a tracked insertion or deletion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TrackedChange {
    /// Stable semantic identity.
    pub id: EntityId,
    /// True for insertion, false for deletion.
    pub insertion: bool,
    /// Authored change text.
    pub text: String,
    /// Author when supplied.
    pub author: Option<String>,
}

/// Represents a structured content control.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentControl {
    /// Stable semantic identity.
    pub id: EntityId,
    /// Adapter-provided tag when available.
    pub tag: Option<String>,
    /// Child semantic object identities.
    pub children: Vec<EntityId>,
}

/// Represents a style reference independently of authored text.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TextStyleRef {
    /// Adapter-neutral style identity.
    pub key: String,
    /// Adapter that owns style resolution.
    pub adapter: String,
}
