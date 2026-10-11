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
    /// # Parameters
    /// - `name`: Operation input.
    ///
    /// # Returns
    /// The requested root result.
    /// # Parameters
    /// - `name`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Reads a named root.
    fn read_root(&self, name: &RootName) -> Option<SnapshotId>;
    /// # Parameters
    /// - `name`: Operation input.
    /// - `expected`: Operation input.
    /// - `new`: Operation input.
    ///
    /// # Returns
    /// The requested root result.
    ///
    /// # Errors
    /// Returns a root backend error.
    /// # Parameters
    /// - `name`: Relevant operation input.
    /// - `expected`: Relevant operation input.
    /// - `new`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Atomically promotes a root when its expected value matches.
    ///
    /// # Errors
    /// Returns a conflict when the expected value is stale.
    fn compare_and_set_root(
        &self,
        name: RootName,
        expected: Option<SnapshotId>,
        new: SnapshotId,
    ) -> Result<(), RootError>;

    /// # Parameters
    /// - `name`: Operation input.
    /// - `snapshot`: Operation input.
    ///
    /// # Returns
    /// The requested root result.
    ///
    /// # Errors
    /// Returns a root backend error.
    /// Unconditionally points a root at an immutable snapshot.
    ///
    /// # Parameters
    /// - `name`: Root name.
    /// - `snapshot`: Immutable snapshot identity.
    ///
    /// # Errors
    /// Returns a backend error when the update cannot be stored.
    fn set_root(&self, name: RootName, snapshot: SnapshotId) -> Result<(), RootError> {
        let expected = self.read_root(&name);
        self.compare_and_set_root(name, expected, snapshot)
    }

    /// # Parameters
    /// - `name`: Operation input.
    /// - `expected`: Operation input.
    /// - `new`: Operation input.
    ///
    /// # Returns
    /// The requested root result.
    ///
    /// # Errors
    /// Returns a root backend error.
    /// Performs compare-and-swap and returns a normal stale outcome.
    ///
    /// # Parameters
    /// - `name`: Root name.
    /// - `expected`: Expected snapshot.
    /// - `new`: Replacement snapshot.
    ///
    /// # Returns
    /// Updated or stale outcome.
    ///
    /// # Errors
    /// Returns backend failures.
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
