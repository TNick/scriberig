//! Finds paragraph text in bounded rich-document views at exact snapshots.

use scriberig_core::{EntityId, ProjectId, SnapshotId};

use crate::{DocumentBlockView, ProjectError, ProjectService};

/// Maximum number of paragraph matches returned by one search page.
pub const MAX_DOCUMENT_SEARCH_PAGE: usize = 100;

/// Identifies one paragraph match in a semantic document search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentParagraphMatch {
    /// Stable semantic paragraph identity.
    pub paragraph: EntityId,
    /// Parent section identity for document-level blocks.
    pub section: EntityId,
    /// Authored run text joined in order.
    pub text: String,
}

/// Contains a bounded page of paragraph matches at one exact snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentParagraphSearch {
    /// Project owning the semantic document.
    pub project: ProjectId,
    /// Exact immutable snapshot used for every match.
    pub snapshot: SnapshotId,
    /// Stable semantic document identity.
    pub document: EntityId,
    /// Matching paragraphs in document order.
    pub matches: Vec<DocumentParagraphMatch>,
    /// Total matches before pagination.
    pub total: usize,
    /// Whether another page contains matches.
    pub has_more: bool,
}

impl ProjectService {
    /// Finds paragraph text at an exact project snapshot.
    ///
    /// Matching is case-insensitive substring search over authored inline run
    /// text; results retain semantic paragraph identities and document order.
    ///
    /// # Parameters
    /// - `project`: Project owning the semantic document.
    /// - `snapshot`: Exact immutable snapshot to inspect.
    /// - `document`: Stable semantic document identity.
    /// - `query`: Non-empty text substring to find.
    /// - `offset`: Zero-based result offset.
    /// - `limit`: Maximum number of matches, from one through one hundred.
    ///
    /// # Returns
    /// Bounded snapshot-bound paragraph matches and continuation state.
    ///
    /// # Errors
    /// Returns stale-snapshot, malformed document, invalid query/page, or
    /// ownership errors.
    pub fn find_document_paragraphs(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        document: EntityId,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> Result<DocumentParagraphSearch, ProjectError> {
        if !(1..=MAX_DOCUMENT_SEARCH_PAGE).contains(&limit) {
            return Err(search_error(
                "invalid_page_size",
                "document search limit must be between one and one hundred",
            ));
        }
        if query.trim().is_empty() {
            return Err(search_error(
                "invalid_query",
                "document search text is empty",
            ));
        }
        let tree = self.document_tree(project, snapshot, document)?;
        let needle = query.to_lowercase();
        let mut matches = Vec::new();
        for section in &tree.sections {
            collect_matches(&section.blocks, section.id, &needle, &mut matches);
        }
        if offset > matches.len() {
            return Err(search_error(
                "offset_out_of_range",
                "document search offset exceeds the matching paragraph count",
            ));
        }
        let total = matches.len();
        let page = matches
            .into_iter()
            .skip(offset)
            .take(limit + 1)
            .collect::<Vec<_>>();
        let has_more = page.len() > limit;
        Ok(DocumentParagraphSearch {
            project,
            snapshot,
            document,
            matches: page.into_iter().take(limit).collect(),
            total,
            has_more,
        })
    }
}

/// Appends matching paragraphs from ordered blocks and nested table cells.
///
/// # Parameters
/// - `blocks`: Ordered public document blocks to search.
/// - `section`: Section identity owning the block sequence.
/// - `needle`: Lowercase non-empty search text.
/// - `matches`: Destination in document order.
///
/// # Returns
/// Unit after matching paragraph records have been appended.
fn collect_matches(
    blocks: &[DocumentBlockView],
    section: EntityId,
    needle: &str,
    matches: &mut Vec<DocumentParagraphMatch>,
) {
    for block in blocks {
        match block {
            DocumentBlockView::Paragraph { paragraph } => {
                let text = paragraph
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>();
                if text.to_lowercase().contains(needle) {
                    matches.push(DocumentParagraphMatch {
                        paragraph: paragraph.id,
                        section,
                        text,
                    });
                }
            }
            DocumentBlockView::Table { table } => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_matches(&cell.blocks, section, needle, matches);
                    }
                }
            }
        }
    }
}

/// Creates a structured semantic document search error.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// Project search error without source package bytes.
fn search_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
