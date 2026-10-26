//! Defines plan values for durable session transport.

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

/// Immutable plan projection retained for one earlier revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanRevision {
    /// Monotonic revision identity.
    pub revision: u64,
    /// Goal at this revision.
    pub goal: String,
    /// Plan state at this revision.
    pub status: SessionPlanStatus,
    /// Nodes retained for inspection at this revision.
    pub nodes: Vec<SessionPlanNode>,
    /// Dependency graph at this revision.
    pub dependencies: Vec<SessionPlanDependency>,
    /// Safe replan reason, when this revision came from a replan.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Plan projection with structural nodes and a separate dependency DAG.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlan {
    /// Stable plan identity.
    pub id: String,
    /// Current durable plan revision.
    pub revision: u64,
    /// Durable plan objective.
    pub goal: String,
    /// Current plan lifecycle state.
    pub status: SessionPlanStatus,
    /// Exact session continuation where this plan was created.
    #[serde(default)]
    pub anchor: Option<SessionContinuationAnchor>,
    /// Root branch from which planning began.
    pub root_branch: String,
    /// Durable work nodes.
    pub nodes: Vec<SessionPlanNode>,
    /// Independent result-based dependency edges.
    pub dependencies: Vec<SessionPlanDependency>,
    /// Branch authorized to coordinate automatic plan advancement.
    #[serde(default)]
    pub coordinator: Option<String>,
    /// Stable plan-node identity currently being coordinated.
    #[serde(default)]
    pub coordinator_cursor: Option<String>,
    /// Unique coordinator operation claim.
    #[serde(default)]
    pub coordinator_claim: Option<SessionCoordinatorClaim>,
    /// Immutable earlier plan revisions.
    #[serde(default)]
    pub history: Vec<SessionPlanRevision>,
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
