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
}

/// Defines backend-neutral named-root operations.
pub trait RootStore {
    /// # Parameters
    /// - `name`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Reads a named root.
    fn read_root(&self, name: &RootName) -> Option<SnapshotId>;
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
}
