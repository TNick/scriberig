//! Defines stable handles returned when TUI agent work is accepted.

use scriberig_core::{EntityId, ProjectId, SnapshotId};

/// Tracks stable handles and optional failure for accepted agent work.
///
/// Public members
/// - `operation`: Shared operation identity.
/// - `session`: Session receiving the operation.
/// - `branch`: Root or continuation branch.
/// - `workspace`: Workspace activated on success.
/// - `failure`: Safe error when execution failed after acceptance.
///
/// Private members: None.
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
    /// Safe failure returned after the application durably accepted this work.
    pub failure: Option<AgentExecutionFailure>,
}

/// Describes a failure for work already accepted into a session.
///
/// Public members
/// - `category`: Stable application error category.
/// - `message`: Sanitized user-facing detail.
///
/// Private members: None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentExecutionFailure {
    /// Stable application error category.
    pub category: String,
    /// Sanitized user-facing failure detail.
    pub message: String,
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
