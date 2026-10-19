//! Defines named root compare-and-set behavior.

use scriberig_core::{RootName, SnapshotId};

/// Reports a named-root update conflict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootError {
    /// The expected value does not match the current root value.
    Conflict {
        /// Root being promoted.
        name: RootName,
        /// Value supplied by the caller.
        expected: Option<SnapshotId>,
        /// Value currently stored by the backend.
        actual: Option<SnapshotId>,
    },
    /// A backend-specific durable storage failure.
    Backend(String),
}

/// Outcome of an optimistic root compare-and-swap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CasOutcome {
    /// The root was advanced.
    Updated,
    /// The expected value was stale.
    Stale {
        /// Value observed by the store.
        actual: Option<SnapshotId>,
    },
}

/// Defines backend-neutral named-root operations.
pub trait RootStore {
    /// - `name`: Operation input.
    ///
    /// The requested root result.
    /// - `name`: Relevant operation input.
    ///
    /// The requested operation result.
    /// Reads a named root.
    /// # Returns
    /// The operation result.
    fn read_root(&self, name: &RootName) -> Option<SnapshotId>;
    /// - `name`: Operation input.
    /// - `expected`: Operation input.
    /// - `new`: Operation input.
    ///
    /// The requested root result.
    ///
    /// Returns a root backend error.
    /// - `name`: Relevant operation input.
    /// - `expected`: Relevant operation input.
    /// - `new`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Atomically promotes a root when its expected value matches.
    ///
    /// Returns a conflict when the expected value is stale.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_set_root(
        &self,
        name: RootName,
        expected: Option<SnapshotId>,
        new: SnapshotId,
    ) -> Result<(), RootError>;

    /// - `name`: Operation input.
    /// - `snapshot`: Operation input.
    ///
    /// The requested root result.
    ///
    /// Returns a root backend error.
    /// Unconditionally points a root at an immutable snapshot.
    ///
    /// - `name`: Root name.
    /// - `snapshot`: Immutable snapshot identity.
    ///
    /// Returns a backend error when the update cannot be stored.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_root(&self, name: RootName, snapshot: SnapshotId) -> Result<(), RootError>;

    /// - `name`: Operation input.
    /// - `expected`: Operation input.
    /// - `new`: Operation input.
    ///
    /// The requested root result.
    ///
    /// Returns a root backend error.
    /// Performs compare-and-swap and returns a normal stale outcome.
    ///
    /// - `name`: Root name.
    /// - `expected`: Expected snapshot.
    /// - `new`: Replacement snapshot.
    ///
    /// Updated or stale outcome.
    ///
    /// Returns backend failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap(
        &self,
        name: RootName,
        expected: Option<SnapshotId>,
        new: SnapshotId,
    ) -> Result<CasOutcome, RootError> {
        match self.compare_and_set_root(name, expected, new) {
            Ok(()) => Ok(CasOutcome::Updated),
            Err(RootError::Conflict { actual, .. }) => Ok(CasOutcome::Stale { actual }),
            Err(error) => Err(error),
        }
    }
}
