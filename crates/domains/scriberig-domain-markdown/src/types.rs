//! Defines the canonical Markdown entity and relation vocabulary.

use scriberig_core::ObjectType;

/// Returns a validated Markdown entity type.
///
/// # Parameters
/// - `name`: Stable suffix in the Markdown entity namespace.
///
/// # Returns
/// The corresponding universal object type.
///
/// # Panics
/// Panics only if a static caller supplies an invalid type suffix.
#[must_use]
pub fn entity_type(name: &str) -> ObjectType {
    ObjectType::new(format!("markdown.{name}")).expect("static Markdown type")
}

/// Stable Markdown entity names required by the first schema slice.
pub const ENTITY_TYPES: &[&str] = &[
    "document",
    "section",
    "heading",
    "paragraph",
    "list",
    "list_item",
    "table",
    "table_row",
    "table_cell",
    "code_block",
    "quote_block",
    "link",
    "image_reference",
    "inline_content",
];

/// Stable ordered containment relation names.
pub const CONTAINMENT_RELATIONS: &[&str] = &[
    "contains.document_block",
    "contains.section_block",
    "contains.section",
    "contains.list_item",
    "contains.table_row",
    "contains.table_cell",
];
