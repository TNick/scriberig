//! Compares Markdown candidates by semantic structure rather than formatting.

use crate::{candidate::CandidateDocument, model::MarkdownDocument};

/// Compares two semantic documents for equality.
///
/// # Parameters
/// - `left`: First semantic document.
/// - `right`: Second semantic document.
///
/// # Returns
/// `true` when the semantic structures match.
#[must_use]
pub fn equivalent(left: &MarkdownDocument, right: &MarkdownDocument) -> bool {
    left == right
}

/// Compares two parsed candidates while ignoring source length and formatting.
///
/// # Parameters
/// - `left`: First parsed candidate.
/// - `right`: Second parsed candidate.
///
/// # Returns
/// `true` when their semantic documents match.
#[must_use]
pub fn semantically_equivalent(left: &CandidateDocument, right: &CandidateDocument) -> bool {
    equivalent(&left.document, &right.document)
}
