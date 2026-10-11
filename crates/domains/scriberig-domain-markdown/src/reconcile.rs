//! Compares Markdown semantic documents without persistence side effects.

use crate::change::{Reconciliation, SemanticChange};
use crate::mapping::MarkdownDocument;
use crate::match_nodes::match_blocks;

/// Reconciles a parsed candidate against a semantic base document.
///
/// # Parameters
/// - `base`: Previously accepted semantic document.
/// - `candidate`: Newly parsed semantic candidate.
///
/// # Returns
/// Structural changes and explicit duplicate-content ambiguity.
#[must_use]
pub fn reconcile(base: &MarkdownDocument, candidate: &MarkdownDocument) -> Reconciliation {
    let (matches, ambiguous) = match_blocks(base, candidate);
    let mut changes = Vec::new();
    let mut matched_base = Vec::new();
    for matched in &matches {
        matched_base.push(matched.base);
        if matched.base != matched.candidate {
            changes.push(SemanticChange::Move {
                base: matched.base,
                candidate: matched.candidate,
            });
        }
    }
    for (candidate_index, _block) in candidate.blocks.iter().enumerate() {
        if matches.iter().all(|item| item.candidate != candidate_index)
            && !ambiguous.contains(&candidate_index)
        {
            if candidate_index < base.blocks.len() && !matched_base.contains(&candidate_index) {
                changes.push(SemanticChange::Revise {
                    base: candidate_index,
                    candidate: candidate_index,
                });
                matched_base.push(candidate_index);
            } else {
                changes.push(SemanticChange::Insert {
                    candidate: candidate_index,
                });
            }
        }
    }
    for base_index in 0..base.blocks.len() {
        if !matched_base.contains(&base_index) {
            changes.push(SemanticChange::Delete { base: base_index });
        }
    }
    changes.sort_by_key(|change| match change {
        SemanticChange::Revise { candidate, .. }
        | SemanticChange::Insert { candidate }
        | SemanticChange::Move { candidate, .. } => *candidate,
        SemanticChange::Delete { base } => *base,
    });
    Reconciliation { changes, ambiguous }
}
