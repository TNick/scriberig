//! Defines parser-owned candidates that do not assign canonical identity.

use crate::model::MarkdownDocument;

/// A parsed Markdown candidate with source-oriented diagnostics metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateDocument {
    /// Semantic candidate tree.
    pub document: MarkdownDocument,
    /// Source byte length used only for diagnostics.
    pub source_length: usize,
}

impl CandidateDocument {
    /// Creates a candidate from a semantic document and source length.
    ///
    /// # Parameters
    /// - `document`: Parsed semantic document.
    /// - `source_length`: Exact source byte length.
    ///
    /// # Returns
    /// A candidate retaining semantic data and diagnostic metadata.
    #[must_use]
    pub fn new(document: MarkdownDocument, source_length: usize) -> Self {
        Self {
            document,
            source_length,
        }
    }
}
