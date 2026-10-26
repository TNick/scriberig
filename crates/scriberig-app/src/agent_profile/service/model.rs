//! Defines public profile candidate results and errors.

use crate::AgentProfileDescriptor;
use scriberig_core::{RevisionId, SnapshotId};

/// A candidate profile change bound to its exact base and resulting snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileCandidate {
    pub(super) base: SnapshotId,
    pub(super) expected_root: Option<SnapshotId>,
    pub(super) snapshot: SnapshotId,
    pub(super) profile: String,
    pub(super) revision: RevisionId,
}

impl ProfileCandidate {
    /// Returns the exact snapshot from which this candidate was proposed.
    #[must_use]
    pub const fn base(&self) -> SnapshotId {
        self.base
    }

    /// Returns the immutable candidate snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> SnapshotId {
        self.snapshot
    }

    /// Returns the stable profile identity affected by this candidate.
    ///
    /// # Returns
    /// The profile identity stored in the candidate.
    #[must_use]
    pub fn profile_identity(&self) -> &str {
        &self.profile
    }

    /// Returns the exact profile revision stored in the candidate.
    #[must_use]
    pub const fn revision(&self) -> RevisionId {
        self.revision
    }
}

/// Exact profile read together with its immutable containing snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileInspection {
    /// Fully resolved descriptor with exact component revisions.
    pub profile: AgentProfileDescriptor,
    /// Snapshot from which the profile and components were read.
    pub snapshot: SnapshotId,
}

/// Reports a profile authorization, storage, identity, or conflict failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentProfileError {
    /// The requested capability is absent from the trusted context.
    Unauthorized {
        /// Requested operation name.
        action: String,
        /// Stable profile identity.
        profile: String,
    },
    /// The requested profile or exact revision was not visible in the snapshot.
    NotFound {
        /// Stable profile identity.
        profile: String,
        /// Exact snapshot that did not contain the requested revision.
        snapshot: SnapshotId,
    },
    /// The application has no provider configured for the requested profile operation.
    Unsupported(String),
    /// A profile or component definition is structurally invalid.
    Invalid(String),
    /// The canonical profile root moved since the operation's base was captured.
    StaleBase {
        /// Snapshot supplied as the candidate base.
        expected: SnapshotId,
        /// Snapshot currently promoted on the profile root, if any.
        actual: Option<SnapshotId>,
    },
    /// A universal store operation failed.
    Store(String),
    /// A retained semantic revision could not be decoded or validated.
    Encoding(String),
}

impl std::fmt::Display for AgentProfileError {
    /// Formats a profile-operation failure for safe application error propagation.
    ///
    /// # Parameters
    /// - `self`: Failure to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns an error if the formatter rejects the message.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AgentProfileError {}

/// Reports the result of a shared profile-store operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentProfileStoreResult {
    /// The profile candidate was promoted.
    Promoted {
        /// Newly promoted exact snapshot.
        snapshot: SnapshotId,
    },
    /// Promotion lost a compare-and-swap race.
    Stale {
        /// Snapshot now stored at the profile root, if any.
        actual: Option<SnapshotId>,
    },
}
