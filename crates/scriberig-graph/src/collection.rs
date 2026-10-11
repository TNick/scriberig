//! Defines ordered and unordered semantic entity collections.

use scriberig_core::EntityId;
use std::collections::BTreeSet;

/// A collection whose order or membership is part of semantic state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionValue {
    /// Sequence preserving member order; duplicate members are allowed.
    Ordered(Vec<EntityId>),
    /// Set preserving membership only; duplicates are removed deterministically.
    Unordered(BTreeSet<EntityId>),
}

impl CollectionValue {
    /// Creates an unordered collection from any member sequence.
    ///
    /// # Parameters
    /// - `members`: Entity identities whose membership is retained.
    ///
    /// # Returns
    /// An unordered collection with deterministic set semantics.
    #[must_use]
    pub fn unordered(members: impl IntoIterator<Item = EntityId>) -> Self {
        Self::Unordered(members.into_iter().collect())
    }
}
