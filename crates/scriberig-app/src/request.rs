//! Defines the transport-independent universal application request model.

use crate::DomainOperationRequest;
use scriberig_core::{RootName, SnapshotId};
use scriberig_query::{FindRequest, FollowRequest, InspectRequest, ListRequest, QuerySpec};

/// Resolves an application request to an exact snapshot before execution.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum SnapshotSelector {
    /// Use this exact immutable snapshot.
    Exact(SnapshotId),
    /// Resolve this named root through application state.
    NamedRoot(RootName),
}

/// Identifies a semantic diff request.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DiffRequest {
    /// Earlier exact snapshot.
    pub from: SnapshotId,
    /// Later exact snapshot.
    pub to: SnapshotId,
}

/// Identifies exact candidates for an explicit three-way semantic merge.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MergeRequest {
    /// Exact common candidate base.
    pub base: SnapshotId,
    /// Exact candidate produced by the first dependency.
    pub left: SnapshotId,
    /// Exact candidate produced by the second dependency.
    pub right: SnapshotId,
}

/// Identifies a history request for a semantic target.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct HistoryRequest {
    /// Snapshot or named root from which history is resolved.
    pub snapshot: SnapshotSelector,
}

/// Identifies a rendering request for a semantic target.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RenderRequest {
    /// Exact snapshot context for rendering.
    pub snapshot: SnapshotSelector,
    /// Registered projection/view identifier.
    pub view: String,
}

/// Identifies a branch operation without implementing branch mutation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct BranchRequest {
    /// Root from which the branch is created.
    pub base: SnapshotSelector,
    /// Requested branch name.
    pub name: RootName,
}

/// Identifies an application change-application request.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ApplyRequest {
    /// Exact base snapshot required by the change.
    pub base_snapshot: SnapshotId,
    /// Optional branch receiving the candidate context.
    pub branch: Option<RootName>,
    /// Stable migration/change handle.
    pub change: String,
}

/// Identifies a commit request for an already validated change.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CommitRequest {
    /// Exact base snapshot being committed.
    pub base_snapshot: SnapshotId,
    /// Named branch whose root is promoted, or `main` when omitted.
    pub branch: Option<RootName>,
    /// Stable migration/change handle.
    pub change: String,
}

/// Selects one universal application operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationRequest {
    /// Inspect an exact semantic target.
    Inspect(InspectRequest),
    /// List a snapshot-bound collection.
    List(ListRequest),
    /// Follow a snapshot-bound relation.
    Follow(FollowRequest),
    /// Discover semantic entities in an exact scope.
    Find(FindRequest),
    /// Evaluate a structural query in an exact scope.
    Query(QuerySpec),
    /// Compute a semantic diff.
    Diff(DiffRequest),
    /// Merge exact candidate snapshots without promoting a root.
    Merge(MergeRequest),
    /// Read semantic history.
    History(HistoryRequest),
    /// Render through a registered view.
    Render(RenderRequest),
    /// Create or update a branch context.
    Branch(BranchRequest),
    /// Apply a proposed change.
    Apply(ApplyRequest),
    /// Commit a validated change.
    Commit(CommitRequest),
    /// Execute a registered exact-target semantic domain operation.
    DomainOperation(DomainOperationRequest),
}
