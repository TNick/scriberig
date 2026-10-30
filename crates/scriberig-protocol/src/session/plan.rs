//! Defines plan values for durable session transport.

pub use super::plan_semantics::*;
use crate::SessionReference;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Durable lifecycle state for one planned work item.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanNodeStatus {
    /// Waiting for explicit prerequisite results.
    Planned,
    /// Ready for a worker to claim.
    Ready,
    /// Running on one session branch.
    Running,
    /// Completed with a durable result.
    Completed,
    /// Failed without producing a dependency result.
    Failed,
    /// Explicitly abandoned.
    Abandoned,
}

/// Upstream provider/model selection assigned to a plan node.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionModelAssignment {
    /// Upstream provider identity, without provider credentials.
    pub provider_id: String,
    /// Provider-scoped upstream model identity.
    pub model_id: String,
    /// Advertised thinking level, when selected.
    #[serde(default)]
    pub thinking_level: Option<String>,
}

/// Durable planned work located in the session tree.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanNode {
    /// Stable plan-node identity.
    pub id: String,
    /// Branch or plan node that structurally owns this node.
    pub parent: String,
    /// Human-readable work title.
    pub title: String,
    /// Focused work instruction.
    pub purpose: String,
    /// Durable scheduling state.
    pub status: SessionPlanNodeStatus,
    /// Exact immutable node revision, when supplied by the shared view.
    #[serde(default)]
    pub exact_revision: Option<String>,
    /// Derived effective status including required descendants and gates.
    #[serde(default)]
    pub effective_status: Option<SessionPlanNodeStatus>,
    /// Whether this node's subtree contains ignored work.
    #[serde(default)]
    pub contains_ignored: bool,
    /// Whether this node's subtree contains abandoned work.
    #[serde(default)]
    pub contains_abandoned: bool,
    /// Typed reason for direct abandonment.
    #[serde(default)]
    pub abandon_reason: Option<SessionPlanAbandonReason>,
    /// Branch executing this work, when started.
    #[serde(default)]
    pub branch: Option<String>,
    /// Explicit result satisfying downstream dependencies.
    #[serde(default)]
    pub result: Option<String>,
    /// Replacement node identity when this node was superseded.
    #[serde(default)]
    pub superseded_by: Option<String>,
    /// Exact result contract required by downstream work.
    #[serde(default)]
    pub expected_result: Option<SessionExpectedResult>,
    /// Selected semantic handles for this work.
    #[serde(default)]
    pub semantic_scope: Vec<String>,
    /// Independent safe model-library assignment.
    #[serde(default)]
    pub assignment: Option<SessionModelAssignment>,
    /// Exclusive worker claim when the node is running.
    #[serde(default)]
    pub claim: Option<SessionWorkerClaim>,
    /// Stable identity of the node's current summary.
    #[serde(default)]
    pub summary_id: Option<String>,
    /// Bounded concise summary preview.
    #[serde(default)]
    pub summary_preview: Option<String>,
    /// Prerequisite node identities currently blocking this node.
    #[serde(default)]
    pub dependency_blockers: Vec<String>,
    /// Entry gates by evaluated state.
    #[serde(default)]
    pub entry_gates: SessionPlanGateCounts,
    /// Completion gates by evaluated state.
    #[serde(default)]
    pub completion_gates: SessionPlanGateCounts,
    /// Node and gate evidence-association count.
    #[serde(default)]
    pub evidence_count: u32,
    /// Whether an explicit retry is currently available.
    #[serde(default)]
    pub can_retry: bool,
    /// Exact reference to the most recent failed result.
    #[serde(default)]
    pub last_failure: Option<SessionPlanEvidenceReference>,
}

/// Result-based dependency edge between two plan nodes.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanDependency {
    /// Node whose result is required.
    pub prerequisite: String,
    /// Node blocked until the result exists.
    pub dependent: String,
    /// Exact durable result that fulfilled the edge.
    #[serde(default)]
    pub fulfilled_by: Option<String>,
}

/// Exact provider-neutral contract for a plan-node result.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionExpectedResult {
    /// Stable result kind expected from the worker.
    pub kind: String,
    /// Whether at least one semantic result handle is required.
    pub requires_handle: bool,
}

/// Durable plan lifecycle projected to clients.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanStatus {
    /// Plan is being authored.
    Draft,
    /// Plan is valid and ready to start.
    Planned,
    /// Coordinator may advance eligible work.
    Running,
    /// Automatic dispatch is paused.
    Paused,
    /// All required work completed.
    Completed,
    /// Plan ended after failure.
    Failed,
    /// Plan was stopped explicitly.
    Cancelled,
}

/// Stable continuation location supplied to protocol clients.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionContinuationAnchor {
    /// Current branch end.
    BranchHead {
        /// Stable branch identity.
        branch: String,
    },
    /// After one externally visible assistant output.
    AssistantOutput {
        /// Stable branch identity.
        branch: String,
        /// Stable assistant-output turn identity.
        turn: String,
    },
    /// After one user input.
    HumanInput {
        /// Stable branch identity.
        branch: String,
        /// Stable human-input turn identity.
        turn: String,
    },
    /// From one committed branch result.
    BranchResult {
        /// Stable branch identity.
        branch: String,
        /// Stable branch-result identity.
        result: String,
    },
}

/// Safe identity and state of the unique automatic coordinator.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionCoordinatorClaim {
    /// Stable claim identity.
    pub id: String,
    /// Shared operation identity when available.
    #[serde(default)]
    pub operation: Option<String>,
    /// Safe runtime owner identifier.
    pub owner: String,
    /// Plan-local claim generation.
    pub generation: u64,
}

/// Safe identity and captured assignment of one active node worker.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionWorkerClaim {
    /// Stable worker claim identity.
    pub id: String,
    /// Claimed plan node.
    pub node: String,
    /// Exclusively executing branch.
    pub branch: String,
    /// Safe runtime owner identity.
    pub owner: String,
    /// Node-local claim generation.
    pub generation: u64,
    /// Safe model assignment captured at claim time.
    #[serde(default)]
    pub assignment: Option<SessionModelAssignment>,
}

/// One structured plan node proposal without application identities.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanNodeDraft {
    /// Stable request-local key used by dependency declarations.
    pub key: String,
    /// Parent proposal index, or the exact anchor branch when absent.
    #[serde(default)]
    pub parent_index: Option<usize>,
    /// Human-readable work title.
    pub title: String,
    /// Focused worker instruction.
    pub purpose: String,
    /// Result contract required by dependents.
    #[serde(default)]
    pub expected_result: Option<SessionExpectedResult>,
    /// Exact semantic handles selected for this node.
    #[serde(default)]
    pub semantic_scope: Vec<String>,
    /// Safe model-library assignment.
    #[serde(default)]
    pub assignment: Option<SessionModelAssignment>,
}

/// Creates a structured durable plan at an exact session anchor.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanCreateRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Exact durable conversation location.
    pub anchor: SessionContinuationAnchor,
    /// Durable plan objective.
    pub goal: String,
    /// Structured proposed work.
    pub nodes: Vec<SessionPlanNodeDraft>,
    /// Request-local dependency index pairs `(prerequisite, dependent)`.
    pub dependencies: Vec<(usize, usize)>,
}

/// Plan lifecycle operation selected by a client.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanAction {
    /// Claim one coordinator and begin advancing.
    Start,
    /// Stop automatic dispatch while preserving state.
    Pause,
    /// Resume automatic dispatch.
    Resume,
    /// Cancel automatic advancement.
    Stop,
    /// Complete after all exact durable results are available.
    Complete,
}

/// Applies one optimistic plan lifecycle mutation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanLifecycleRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Plan revision observed by the client.
    pub expected_revision: u64,
    /// Requested lifecycle operation.
    pub action: SessionPlanAction,
}

/// Retries one failed plan node at an observed plan revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanRetryRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Stable failed node identity.
    pub node: String,
    /// Plan revision observed by the client.
    pub expected_revision: u64,
}

/// Reads one bounded page of worker attempts at an exact plan snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanAttemptHistoryRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Stable plan-node identity.
    pub node: String,
    /// Exact semantic snapshot identity.
    pub snapshot: String,
    /// Number of matching attempts to skip in stable order.
    pub offset: usize,
    /// Maximum number of attempts to return.
    pub limit: usize,
}

/// Reads one bounded page of gates and evidence at an exact plan snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanNodeDetailsRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Stable plan-node identity.
    pub node: String,
    /// Exact universal snapshot identity.
    pub snapshot: String,
    /// Number of matching records to skip in each category.
    pub offset: usize,
    /// Maximum records to return in each category.
    pub limit: usize,
}

/// Changes one unstarted node's safe model assignment.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanAssignmentRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Stable node identity.
    pub node: String,
    /// Plan revision observed by the client.
    pub expected_revision: u64,
    /// New assignment or `None` to inherit defaults.
    #[serde(default)]
    pub assignment: Option<SessionModelAssignment>,
}

/// Identifies a current or proposed node in a replan.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanNodeReference {
    /// Existing durable node identity.
    Existing {
        /// Stable plan node identity.
        id: String,
    },
    /// Request-local key for a proposed node.
    Proposed {
        /// Stable key from the additions list.
        key: String,
    },
}

/// Applies one complete optimistic plan revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanReplanRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Plan revision observed by the client.
    pub expected_revision: u64,
    /// New structured work nodes.
    pub additions: Vec<SessionPlanNodeDraft>,
    /// New result dependency edges.
    pub dependencies: Vec<(SessionPlanNodeReference, SessionPlanNodeReference)>,
    /// Existing dependency edges to remove.
    pub removed_dependencies: Vec<(String, String)>,
    /// Existing nodes and proposed replacement keys.
    pub superseded: Vec<(String, String)>,
    /// Safe assignment changes for existing unstarted nodes.
    pub assignment_changes: Vec<(String, Option<SessionModelAssignment>)>,
    /// Completed terminal nodes explicitly preserved by the proposal.
    pub preserved_terminal_nodes: Vec<String>,
    /// Safe reason recorded in plan history.
    pub reason: String,
}
