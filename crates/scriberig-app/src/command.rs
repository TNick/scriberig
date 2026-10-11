//! Defines the command-side application boundary without implementing mutations.

use scriberig_core::SnapshotId;

/// Identifies a command family exposed by the application layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandKind {
    /// Applies a migration against an explicit base snapshot.
    ApplyMigration,
}

/// Describes the immutable base required by a future migration command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandContext {
    /// Exact snapshot from which the command was proposed.
    pub base_snapshot: SnapshotId,
}
