//! Defines parser-owned candidates that do not assign canonical identity.

use crate::model::MarkdownDocument;

/// A parsed Markdown candidate with source-oriented diagnostics metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateDocument {
    /// Semantic candidate tree.
    pub document: MarkdownDocument,
    /// Source byte length used only for diagnostics.
    pub source_length: usize,
    /// Local structural paths for candidate-node diagnostics.
    pub node_paths: Vec<Vec<usize>>,
    /// Source-coordinate spans for parser events that define supported blocks.
    pub source_spans: Vec<SourceSpan>,
}

/// A byte range in the external Markdown source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    /// Inclusive byte offset of the span start.
    pub start: usize,
    /// Exclusive byte offset of the span end.
    pub end: usize,
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
            node_paths: node_paths(&document),
            document,
            source_length,
            source_spans: Vec::new(),
        }
    }

    /// Creates a candidate with parser source coordinates.
    ///
    /// # Parameters
    /// - `document`: Parsed semantic document.
    /// - `source_length`: Exact source byte length.
    /// - `source_spans`: Supported block-coordinate spans.
    ///
    /// # Returns
    /// A candidate retaining semantic and diagnostic metadata.
    #[must_use]
    pub fn with_source_spans(
        document: MarkdownDocument,
        source_length: usize,
        source_spans: Vec<SourceSpan>,
    ) -> Self {
        Self {
            node_paths: node_paths(&document),
            document,
            source_length,
            source_spans,
        }
    }
}

/// Collects local candidate paths without assigning canonical identity.
/// - `document`: Argument named `document`.
///
/// # Returns
/// The value produced by the declaration.
fn node_paths(document: &MarkdownDocument) -> Vec<Vec<usize>> {
    let mut paths = Vec::new();
    for (index, block) in document.blocks.iter().enumerate() {
        collect_paths(block, &[index], &mut paths);
    }
    paths
}

/// Adds one block and its nested section children to candidate paths.
/// - `block`: Argument named `block`.
/// - `path`: Argument named `path`.
/// - `paths`: Argument named `paths`.
fn collect_paths(block: &crate::MarkdownBlock, path: &[usize], paths: &mut Vec<Vec<usize>>) {
    paths.push(path.to_vec());
    if let crate::MarkdownBlock::Section(section) = block {
        for (index, child) in section.blocks.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(index);
            collect_paths(child, &child_path, paths);
        }
    } else if let crate::MarkdownBlock::List { items, .. } = block {
        for (index, item) in items.iter().enumerate() {
            let mut item_path = path.to_vec();
            item_path.push(index);
            paths.push(item_path.clone());
            for (block_index, child) in item.blocks.iter().enumerate() {
                let mut child_path = item_path.clone();
                child_path.push(block_index);
                collect_paths(child, &child_path, paths);
            }
        }
    }
}
