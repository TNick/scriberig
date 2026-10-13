//! Defines stable handles returned when TUI agent work is accepted.

use scriberig_core::{EntityId, ProjectId, SnapshotId};

/// Tracks the stable handles returned when agent work is accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentHandle {
    /// Shared operation identity.
    pub operation: scriberig_core::OperationId,
    /// Session receiving the operation.
    pub session: EntityId,
    /// Root or continuation branch, when known immediately.
    pub branch: Option<EntityId>,
    /// Project workspace activated by a completed onboarding operation.
    pub workspace: Option<AgentWorkspace>,
}

/// Identifies the semantic project context produced by an agent operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentWorkspace {
    /// Stable project identity.
    pub project: ProjectId,
    /// Human-readable project name.
    pub name: String,
    /// Exact immutable project snapshot.
    pub snapshot: SnapshotId,
    /// Semantic source root for navigation.
    pub root: EntityId,
    /// Exact reviewed manifest imported into this workspace.
    pub manifest_id: String,
}
