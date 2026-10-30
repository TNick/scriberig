//! Defines typed rich-document read and edit candidate contracts.

use crate::ArtifactEditCapability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests an exact-snapshot semantic document tree.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentInspectRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic document root identity.
    pub document: String,
}

/// Returns one ordered semantic document tree without package bytes.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentInspectResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot used by the read.
    pub snapshot: String,
    /// Stable semantic document identity.
    pub document: String,
    /// Edit capability of the exact source artifact, when catalogued.
    pub artifact_capability: Option<ArtifactEditCapability>,
    /// Named document styles used by paragraph and run references.
    pub styles: Vec<DocumentStyleView>,
    /// Ordered document sections.
    pub sections: Vec<DocumentSectionView>,
}

/// Describes one named, format-owned style definition.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentStyleView {
    /// Stable style key within the source format.
    pub key: String,
    /// Human-readable style name when supplied by the source.
    pub name: Option<String>,
}

/// Requests bounded paragraph text search at an exact document snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentParagraphFindRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic document identity.
    pub document: String,
    /// Non-empty case-insensitive substring to match.
    pub query: String,
    /// Zero-based match offset.
    pub offset: usize,
    /// Maximum matches requested, at most one hundred.
    pub limit: usize,
}

/// Describes one semantic paragraph match.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentParagraphMatch {
    /// Stable semantic paragraph identity.
    pub paragraph: String,
    /// Parent document section identity.
    pub section: String,
    /// Authored inline text joined in order.
    pub text: String,
}

/// Returns bounded snapshot-bound paragraph matches in document order.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentParagraphFindResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot used for the search.
    pub snapshot: String,
    /// Stable semantic document identity.
    pub document: String,
    /// Matching paragraph identities and text.
    pub matches: Vec<DocumentParagraphMatch>,
    /// Total matching paragraphs before pagination.
    pub total: usize,
    /// Whether another result page exists.
    pub has_more: bool,
}

/// Represents one ordered document section.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentSectionView {
    /// Stable section identity.
    pub id: String,
    /// Ordered paragraphs and tables.
    pub blocks: Vec<DocumentBlockView>,
}

/// Represents one typed semantic paragraph or table block.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocumentBlockView {
    /// Addressable paragraph with ordered runs.
    Paragraph {
        /// Paragraph semantic content.
        paragraph: DocumentParagraphView,
    },
    /// Addressable table with ordered rows and cells.
    Table {
        /// Table semantic content.
        table: DocumentTableView,
    },
}

/// Represents an addressable document paragraph.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentParagraphView {
    /// Stable paragraph identity.
    pub id: String,
    /// Ordered inline text runs.
    pub runs: Vec<DocumentInlineRunView>,
    /// Optional paragraph formatting reference.
    pub style: Option<DocumentTextStyleRef>,
}

/// Represents an ordered text run and its references.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentInlineRunView {
    /// Stable run identity.
    pub id: String,
    /// Authored run text.
    pub text: String,
    /// Optional run formatting reference.
    pub style: Option<DocumentTextStyleRef>,
    /// Optional external or internal hyperlink destination.
    pub hyperlink: Option<String>,
}

/// Identifies a document style without embedding style implementation details.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentTextStyleRef {
    /// Adapter-owned style key.
    pub key: String,
    /// Adapter that resolves the style.
    pub adapter: String,
}

/// Represents an ordered table and its rows.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentTableView {
    /// Stable table identity.
    pub id: String,
    /// Authored row order.
    pub rows: Vec<DocumentRowView>,
}

/// Represents one table row and its cells.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentRowView {
    /// Stable row identity.
    pub id: String,
    /// Authored cell order.
    pub cells: Vec<DocumentCellView>,
}

/// Represents one table cell and its ordered blocks.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentCellView {
    /// Stable table cell identity.
    pub id: String,
    /// Ordered semantic content blocks.
    pub blocks: Vec<DocumentBlockView>,
}

/// Requests an exact-base semantic paragraph replacement candidate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentParagraphEditRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable base snapshot.
    pub snapshot: String,
    /// Stable semantic paragraph identity.
    pub paragraph: String,
    /// Replacement text authored for the paragraph.
    pub replacement: String,
}

/// Requests replacement of one simple table cell through its cell identity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentTableCellEditRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable base snapshot.
    pub snapshot: String,
    /// Stable semantic table-cell identity.
    pub cell: String,
    /// Replacement text authored for the cell.
    pub replacement: String,
}

/// Returns a reviewable document paragraph candidate handle.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentParagraphEditResponse {
    /// Stable project identity.
    pub project: String,
    /// Candidate identity used by shared validation and review operations.
    pub candidate_id: String,
    /// Exact base snapshot used to create the candidate.
    pub base_snapshot: String,
    /// Project-relative DOCX path.
    pub path: String,
    /// Candidate validation state.
    pub validation: DocumentCandidateValidation,
}

/// Requests an exact-base semantic paragraph insertion candidate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentParagraphInsertRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable base snapshot.
    pub snapshot: String,
    /// Stable semantic section identity receiving the paragraph.
    pub section: String,
    /// Authored text for the new paragraph.
    pub text: String,
}

/// Returns a reviewable inserted-paragraph candidate handle.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DocumentParagraphInsertResponse {
    /// Stable project identity.
    pub project: String,
    /// Candidate identity used by shared validation and review operations.
    pub candidate_id: String,
    /// Exact base snapshot used to create the candidate.
    pub base_snapshot: String,
    /// Project-relative DOCX path.
    pub path: String,
    /// Candidate validation state.
    pub validation: DocumentCandidateValidation,
}

/// Describes document candidate validation without exposing package bytes.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DocumentCandidateValidation {
    /// Candidate has not yet been validated in complete project state.
    Pending,
    /// Candidate passed semantic and package validation.
    Passed,
    /// Candidate failed validation with bounded diagnostics.
    Failed {
        /// Safe validation diagnostics.
        diagnostics: Vec<String>,
    },
}
