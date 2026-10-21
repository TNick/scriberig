//! Defines the structured universal application result model.

use scriberig_core::{RevisionId, RootName, SnapshotId};
use scriberig_query::{FindResult, FollowResult, Inspection, ListResult, QueryResult};

/// Describes one immutable entity transition in a migration proposal.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MigrationDelta {
    /// Entity whose canonical revision changes.
    pub entity: scriberig_core::EntityId,
    /// Revision in the base snapshot, when present.
    pub before: Option<RevisionId>,
    /// Revision in the candidate snapshot, when present.
    pub after: Option<RevisionId>,
}

/// Exposes the canonical, snapshot-bound detail of one migration proposal.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MigrationDetail {
    /// Stable migration handle.
    pub id: String,
    /// Exact snapshot from which the proposal starts.
    pub base: SnapshotId,
    /// Exact candidate snapshot produced by the proposal.
    pub result: SnapshotId,
    /// Deterministically ordered entity transitions.
    pub deltas: Vec<MigrationDelta>,
}

/// Summarizes a semantic operation result without flattening it to text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationSummary {
    /// Stable operation or result handle.
    pub handle: String,
    /// Exact snapshot involved when known.
    pub snapshot: Option<SnapshotId>,
    /// Exact revision involved when known.
    pub revision: Option<RevisionId>,
    /// Named branch context used by the operation, when applicable.
    pub branch: Option<RootName>,
    /// Structured semantic diff when this is a diff operation.
    pub diff: Option<sr_change::SemanticDiff>,
    /// Structured migration history when this is a history operation.
    pub history: Option<Vec<sr_change::MigrationReference>>,
    /// Structured rendered projection when this is a render operation.
    pub rendered: Option<sr_projection::RenderedProjection>,
    /// Validation outcome when the operation performed validation.
    pub validation: Option<crate::ValidationStatus>,
    /// Registered policy that produced the current validation outcome.
    pub validation_profile: Option<String>,
    /// Safe validation diagnostics associated with the operation.
    pub validation_diagnostics: Vec<String>,
    /// Non-conflicting presentation details for candidate review.
    pub review_details: Vec<String>,
    /// Safe conflict summaries associated with the operation.
    pub conflicts: Vec<String>,
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
    /// Result of explicit semantic candidate integration.
    Merge(sr_change::MergeResult),
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
    /// Candidate change handle returned by a registered semantic operation.
    DomainOperation(crate::CapabilityExecution),
    /// Reference to a long-running operation.
    OperationStarted(OperationStarted),
}
