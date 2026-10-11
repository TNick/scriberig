//! Defines the structured universal application result model.

use scriberig_core::{RevisionId, SnapshotId};
use scriberig_query::{FindResult, FollowResult, Inspection, ListResult, QueryResult};

/// Summarizes a semantic operation result without flattening it to text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationSummary {
    /// Stable operation or result handle.
    pub handle: String,
    /// Exact snapshot involved when known.
    pub snapshot: Option<SnapshotId>,
    /// Exact revision involved when known.
    pub revision: Option<RevisionId>,
}

/// Identifies a tracked operation that has started.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationStarted {
    /// Stable tracked operation identifier.
    pub operation: crate::OperationId,
    /// Universal operation family.
    pub kind: crate::OperationKind,
}

/// Returns one structured universal application result.
#[derive(Clone, Debug, PartialEq)]
pub enum ApplicationResult {
    /// Result of entity inspection.
    Inspect(Inspection),
    /// Result of collection listing.
    List(ListResult),
    /// Result of relation traversal.
    Follow(FollowResult),
    /// Result of semantic discovery.
    Find(FindResult),
    /// Result of structural query evaluation.
    Query(QueryResult),
    /// Result of semantic diff computation.
    Diff(OperationSummary),
    /// Result of history lookup.
    History(OperationSummary),
    /// Result of projection rendering.
    Render(OperationSummary),
    /// Result of a branch operation.
    Branch(OperationSummary),
    /// Result of applying a change.
    Apply(OperationSummary),
    /// Result of committing a change.
    Commit(OperationSummary),
    /// Reference to a long-running operation.
    OperationStarted(OperationStarted),
}
