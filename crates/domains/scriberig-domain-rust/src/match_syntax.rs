//! Matches parsed Rust syntax using declaration and local structural evidence.

use scriberig_core::EntityId;

use crate::syntax::{RustFile, RustItem};

/// Canonical Rust item with stable identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalRustItem {
    /// Stable canonical entity identity.
    pub id: EntityId,
    /// Canonical syntax item.
    pub item: RustItem,
    /// Semantic declaration position.
    pub position: usize,
}

/// Evidence used to preserve a Rust item identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntaxMatchEvidence {
    /// Exact canonical syntax equality.
    Exact,
    /// Unique declaration name and kind match.
    DeclarationContext,
}

/// Result for one candidate item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyntaxMatch {
    /// Candidate maps to an existing item.
    Matched {
        /// Candidate item index.
        candidate: usize,
        /// Existing item identity.
        existing: EntityId,
        /// Evidence supporting continuity.
        evidence: SyntaxMatchEvidence,
    },
    /// Candidate is new.
    New {
        /// Candidate item index.
        candidate: usize,
    },
    /// Several existing items are equally plausible.
    Ambiguous {
        /// Candidate item index.
        candidate: usize,
        /// Possible existing identities.
        possible: Vec<EntityId>,
    },
}

/// Complete item matching report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxMatchReport {
    /// Candidate item results.
    pub matches: Vec<SyntaxMatch>,
    /// Existing items absent from the candidate.
    pub unmatched: Vec<EntityId>,
}

/// Matches top-level Rust declarations without using source coordinates.
///
/// - `base`: Canonical Rust items.
/// - `candidate`: Newly parsed Rust source.
///
/// Deterministic identity matches and explicit ambiguity.
#[must_use]
/// # Returns
/// The operation result.
pub fn match_items(base: &[CanonicalRustItem], candidate: &RustFile) -> SyntaxMatchReport {
    let mut used = vec![false; base.len()];
    let mut matches = Vec::new();
    for (candidate_index, item) in candidate.items.iter().enumerate() {
        let exact: Vec<_> = base
            .iter()
            .enumerate()
            .filter(|(index, old)| !used[*index] && old.item == *item)
            .map(|(index, _)| index)
            .collect();
        if exact.len() == 1 {
            let index = exact[0];
            used[index] = true;
            matches.push(SyntaxMatch::Matched {
                candidate: candidate_index,
                existing: base[index].id,
                evidence: SyntaxMatchEvidence::Exact,
            });
            continue;
        }
        let contextual: Vec<_> = base
            .iter()
            .enumerate()
            .filter(|(index, old)| !used[*index] && same_declaration(old.item_ref(), item))
            .map(|(index, _)| index)
            .collect();
        match contextual.len() {
            1 => {
                let index = contextual[0];
                used[index] = true;
                matches.push(SyntaxMatch::Matched {
                    candidate: candidate_index,
                    existing: base[index].id,
                    evidence: SyntaxMatchEvidence::DeclarationContext,
                });
            }
            count if count > 1 => matches.push(SyntaxMatch::Ambiguous {
                candidate: candidate_index,
                possible: contextual.iter().map(|index| base[*index].id).collect(),
            }),
            _ => matches.push(SyntaxMatch::New {
                candidate: candidate_index,
            }),
        }
    }
    SyntaxMatchReport {
        matches,
        unmatched: base
            .iter()
            .enumerate()
            .filter(|(index, _)| !used[*index])
            .map(|(_, item)| item.id)
            .collect(),
    }
}

/// Returns a reference to the syntax item in a canonical wrapper.
trait ItemRef {
    /// Returns the wrapped item.
    ///
    /// The canonical syntax item reference.
    /// # Returns
    /// The operation result.
    fn item_ref(&self) -> &RustItem;
}

impl ItemRef for CanonicalRustItem {
    /// Returns the wrapped item.
    ///
    /// The canonical syntax item reference.
    /// # Returns
    /// The operation result.
    fn item_ref(&self) -> &RustItem {
        &self.item
    }
}

/// Reports whether two declarations have stable matching context.
///
/// - `left`: Existing syntax item.
/// - `right`: Candidate syntax item.
///
/// `true` when declaration kind and identity context match.
/// # Returns
/// The operation result.
fn same_declaration(left: &RustItem, right: &RustItem) -> bool {
    match (left, right) {
        (RustItem::Function(left), RustItem::Function(right)) => left.name == right.name,
        (RustItem::Struct(left), RustItem::Struct(right))
        | (RustItem::Enum(left), RustItem::Enum(right))
        | (RustItem::Impl(left), RustItem::Impl(right)) => {
            declaration_head(&left.tokens) == declaration_head(&right.tokens)
        }
        (RustItem::Macro(left), RustItem::Macro(right)) => left.path == right.path,
        (RustItem::Other(left), RustItem::Other(right)) => left == right,
        _ => false,
    }
}

/// Extracts a stable declaration prefix without source coordinates.
///
/// - `tokens`: Canonical declaration token text.
///
/// Declaration tokens before the body.
/// # Returns
/// The operation result.
fn declaration_head(tokens: &str) -> &str {
    tokens.split('{').next().unwrap_or(tokens).trim()
}
