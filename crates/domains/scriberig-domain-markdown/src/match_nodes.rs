//! Defines deterministic Markdown block matching.

use crate::mapping::MarkdownDocument;

/// One candidate-to-base block match.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockMatch {
    /// Candidate block index.
    pub candidate: usize,
    /// Base block index.
    pub base: usize,
}

/// Matches equal semantic blocks and reports duplicate ambiguity.
///
/// - `base`: Existing semantic document.
/// - `candidate`: Parsed candidate document.
///
/// Matches and ambiguous candidate indexes.
#[must_use]
/// # Returns
/// The operation result.
pub fn match_blocks(
    base: &MarkdownDocument,
    candidate: &MarkdownDocument,
) -> (Vec<BlockMatch>, Vec<usize>) {
    let mut matches = Vec::new();
    let mut ambiguous = Vec::new();
    for (candidate_index, block) in candidate.blocks.iter().enumerate() {
        let choices: Vec<_> = base
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, old)| *old == block)
            .map(|(index, _)| index)
            .collect();
        if choices.len() == 1 {
            matches.push(BlockMatch {
                candidate: candidate_index,
                base: choices[0],
            });
        } else if choices.len() > 1 {
            ambiguous.push(candidate_index);
        }
    }
    (matches, ambiguous)
}
