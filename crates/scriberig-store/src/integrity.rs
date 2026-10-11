//! Defines structured storage integrity reports.

use scriberig_core::{RevisionId, SnapshotId};

/// Machine-readable integrity issue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntegrityIssue {
    /// Stored revision content does not hash to its identity.
    RevisionMismatch(RevisionId),
    /// A referenced snapshot is missing or unreadable.
    MissingSnapshot(SnapshotId),
    /// A snapshot-map invariant is invalid.
    InvalidSnapshot(SnapshotId),
    /// A named root points to invalid state.
    InvalidRoot(String),
}

/// Separates corruption from valid unreachable maintenance candidates.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IntegrityReport {
    /// Detected corruption or missing authoritative data.
    pub corrupt: Vec<IntegrityIssue>,
    /// Valid immutable objects not protected by roots.
    pub unreachable: crate::UnreachableObjects,
}

impl IntegrityReport {
    /// Reports whether any corruption was found.
    ///
    /// # Returns
    /// `true` when at least one corruption issue exists.
    #[must_use]
    pub fn is_corrupt(&self) -> bool {
        !self.corrupt.is_empty()
    }
}
