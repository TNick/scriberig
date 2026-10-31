//! Defines shared requests and mutations for durable plans.

use scriberig_core::{EntityId, SnapshotId};

/// Describes one proposed node before application identities are allocated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanNodeDraft {
    /// Stable request-local key used by dependency declarations.
    pub key: String,
    /// Optional parent draft index; absent nodes attach to the anchor branch.
    pub parent_index: Option<usize>,
    /// Human-readable work title.
    pub title: String,
    /// Focused work instruction.
    pub purpose: String,
    /// Expected provider-neutral result contract.
    pub expected_result: Option<scriberig_session::ExpectedResult>,
    /// Semantic handles selected for this work.
    pub semantic_scope: Vec<String>,
    /// Optional safe model-library assignment.
    pub assignment: Option<scriberig_session::ModelAssignment>,
}

/// Requests durable plan creation at an exact session anchor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanCreateRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Exact conversation location where the plan is created.
    pub anchor: scriberig_session::ContinuationAnchor,
    /// Durable plan goal.
    pub goal: String,
    /// Structured untrusted proposal nodes.
    pub nodes: Vec<PlanNodeDraft>,
    /// Dependency index pairs `(prerequisite, dependent)`.
    pub dependencies: Vec<(usize, usize)>,
}

/// Requests one optimistic plan lifecycle mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanLifecycleRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
}

/// Requests an optimistic retry of one durably failed plan node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRetryRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Stable plan-node identity.
    pub node: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
}

/// Requests a bounded page of worker attempts at an exact plan snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanAttemptHistoryRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Stable plan-node identity.
    pub node: EntityId,
    /// Exact semantic snapshot to resolve.
    pub snapshot: SnapshotId,
    /// Number of matching attempts to skip in stable order.
    pub offset: usize,
    /// Maximum number of matching attempts to return.
    pub limit: usize,
}

/// Requests a bounded page of gate and evidence details at an exact snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanNodeDetailsRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Stable plan-node identity.
    pub node: EntityId,
    /// Exact semantic snapshot to resolve.
    pub snapshot: SnapshotId,
    /// Number of matching records to skip in each category.
    pub offset: usize,
    /// Maximum gates and evidence records returned in each category.
    pub limit: usize,
}

/// Selects a durable lifecycle transition for a plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanLifecycleAction {
    /// Enables auto-advancing coordination.
    Start,
    /// Pauses dispatch while preserving plan state.
    Pause,
    /// Resumes dispatch for a paused plan.
    Resume,
    /// Stops auto-advancement.
    Stop,
    /// Completes only after explicit durable node results are present.
    Complete,
}

/// Requests a safe model assignment change for one plan node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanAssignmentRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Stable plan-node identity.
    pub node: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
    /// New library-validated assignment, or `None` to inherit defaults.
    pub assignment: Option<scriberig_session::ModelAssignment>,
}

/// Identifies an existing or newly proposed node in a replan request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanNodeReference {
    /// Existing durable node identity.
    Existing(EntityId),
    /// Request-local key for a proposed node.
    Proposed(String),
}

/// Requests one optimistic durable plan revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanReplanRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
    /// New structured nodes to append.
    pub additions: Vec<PlanNodeDraft>,
    /// Explicit dependency references `(prerequisite, dependent)`.
    pub dependencies: Vec<(PlanNodeReference, PlanNodeReference)>,
    /// Existing dependency edges to remove explicitly.
    pub removed_dependencies: Vec<(EntityId, EntityId)>,
    /// Existing node and proposed replacement key pairs.
    pub superseded: Vec<(EntityId, String)>,
    /// Safe assignment changes for existing unstarted nodes.
    pub assignment_changes: Vec<(EntityId, Option<scriberig_session::ModelAssignment>)>,
    /// Completed terminal node identities that the proposal preserves.
    pub preserved_terminal_nodes: Vec<EntityId>,
    /// Safe explanation or trigger recorded in revision history.
    pub reason: String,
}

/// Requests one optimistic semantic mutation of a durable plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanMutationRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
    /// Typed mutation to apply atomically.
    pub mutation: PlanMutation,
}

/// Selects one semantic plan mutation owned by the shared application layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanMutation {
    /// Appends ordinary work to a running plan.
    AddNextStep(PlanNodeDraft),
    /// Adds structural descendants and optional completion obligations.
    Decompose {
        /// Existing parent node.
        parent: EntityId,
        /// New child work items.
        children: Vec<PlanNodeDraft>,
        /// Whether every child must complete before its parent.
        required: bool,
    },
    /// Adds a typed gate to one node.
    AddGate {
        /// Node whose lifecycle is constrained.
        owner: EntityId,
        /// Gate evaluation phase.
        phase: scriberig_session::PlanGatePhase,
        /// Typed requirement without an executable expression.
        requirement: scriberig_session::PlanGateRequirement,
    },
    /// Retains an explicit waiver on a gate.
    WaiveGate {
        /// Stable gate identity.
        gate: EntityId,
        /// Safe reason for the waiver.
        reason: String,
    },
    /// Associates an exact semantic reference with a node or gate.
    AttachEvidence {
        /// Node or gate receiving the evidence.
        owner: scriberig_session::PlanEvidenceOwner,
        /// Evidence role.
        role: scriberig_session::PlanEvidenceRole,
        /// Exact source evidence.
        evidence: scriberig_session::PlanEvidenceReference,
        /// Optional bounded safe note.
        note: Option<String>,
    },
    /// Replaces one node's current summary.
    UpdateSummary {
        /// Node summarized by the record.
        node: EntityId,
        /// Concise safe summary text.
        text: String,
        /// Exact source revisions or snapshots used by the summary.
        sources: Vec<scriberig_session::PlanSemanticReference>,
    },
    /// Retries one failed node after dependency and gate checks.
    RetryFailedNode {
        /// Failed node identity.
        node: EntityId,
    },
    /// Recovers a failed plan into a paused state for explicit repair.
    RecoverFailedPlan,
}
