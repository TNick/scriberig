//! Defines exact domain-neutral dependencies used to establish result validity.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Describes one exact input or scope dependency of a derivation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Dependency {
    /// An exact entity revision dependency.
    ExactRevision {
        /// Stable conceptual entity identity.
        entity: EntityId,
        /// Exact immutable revision identity.
        revision: RevisionId,
    },
    /// A complete immutable snapshot dependency.
    Snapshot(SnapshotId),
    /// A completeness dependency over an opaque scope or collection ID.
    Scope(String),
    /// An exact semantic configuration dependency.
    Configuration(String),
    /// A dependency on another immutable result handle.
    DerivedResult(RevisionId),
}

/// Stores a normalized, deterministically ordered dependency set.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DependencySet(Vec<Dependency>);

// Normalize dependency order and duplicates at construction.
impl DependencySet {
    /// Creates a dependency set, removing duplicate entries deterministically.
    ///
    /// - `dependencies`: Dependencies in any insertion order.
    ///
    /// A sorted set with duplicates removed.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(mut dependencies: Vec<Dependency>) -> Self {
        dependencies.sort();
        dependencies.dedup();
        Self(dependencies)
    }

    /// Borrows the normalized dependencies in deterministic order.
    ///
    /// - `self`: Dependency set to inspect.
    ///
    /// The ordered dependency slice.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_slice(&self) -> &[Dependency] {
        &self.0
    }
}
