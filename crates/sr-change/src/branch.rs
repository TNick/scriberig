//! Defines branch metadata and branch references.

use crate::history::MigrationId;
use scriberig_core::SnapshotId;

/// Stable identity and compact metadata for one branch line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Branch {
    /// Stable branch identity.
    pub id: String,
    /// Snapshot from which the branch was created.
    pub base: SnapshotId,
    /// Current branch head snapshot.
    pub current: SnapshotId,
    /// Optional navigation title.
    pub title: Option<String>,
    /// Optional purpose.
    pub purpose: Option<String>,
    /// Optional short description.
    pub description: Option<String>,
    /// Optional outcome summary.
    pub outcome: Option<String>,
    /// Optional lifecycle status.
    pub status: Option<String>,
    /// Optional resulting migration reference.
    pub resulting_migration: Option<MigrationId>,
}
