//! Compares Markdown candidates by semantic structure rather than formatting.

use crate::{candidate::CandidateDocument, model::MarkdownDocument};

/// Compares two semantic documents for equality.
///
/// - `left`: First semantic document.
/// - `right`: Second semantic document.
///
/// `true` when the semantic structures match.
#[must_use]
/// # Returns
/// The operation result.
pub fn equivalent(left: &MarkdownDocument, right: &MarkdownDocument) -> bool {
    left == right
}

/// Compares two parsed candidates while ignoring source length and formatting.
///
/// - `left`: First parsed candidate.
/// - `right`: Second parsed candidate.
///
/// `true` when their semantic documents match.
#[must_use]
/// # Returns
/// The operation result.
pub fn semantically_equivalent(left: &CandidateDocument, right: &CandidateDocument) -> bool {
    equivalent(&left.document, &right.document)
}
