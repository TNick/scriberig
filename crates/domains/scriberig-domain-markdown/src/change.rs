//! Defines Markdown semantic reconciliation changes.

/// One structural or content change between semantic documents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticChange {
    /// Existing block content changed.
    Revise {
        /// Base block index.
        base: usize,
        /// Candidate block index.
        candidate: usize,
    },
    /// A new block was inserted.
    Insert {
        /// Candidate block index.
        candidate: usize,
    },
    /// A base block disappeared.
    Delete {
        /// Base block index.
        base: usize,
    },
    /// An unchanged block moved.
    Move {
        /// Base block index.
        base: usize,
        /// Candidate block index.
        candidate: usize,
    },
}

/// Reconciliation output with explicit ambiguity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reconciliation {
    /// Deterministic semantic changes.
    pub changes: Vec<SemanticChange>,
    /// Candidate indexes with duplicate equally plausible predecessors.
    pub ambiguous: Vec<usize>,
}
