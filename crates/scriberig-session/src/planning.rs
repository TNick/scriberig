//! Defines durable plans, assignments, dependencies, and result readiness.

use scriberig_core::{EntityId, OperationId};

/// Represents lifecycle state for a durable plan node.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanNodeStatus {
    /// Work is waiting for required dependency results.
    Planned,
    /// All declared dependency results are available.
    Ready,
    /// A worker currently owns the node's branch.
    Running,
    /// Work completed and produced an explicit result.
    Completed,
    /// Work failed without satisfying downstream dependencies.
    Failed,
    /// Work was explicitly cancelled or abandoned.
    Abandoned,
}

/// Represents the lifecycle of a durable plan.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanStatus {
    /// Plan is being authored and cannot auto advance.
    #[default]
    Draft,
    /// Valid plan is ready to start.
    Planned,
    /// Plan coordinator may dispatch eligible work.
    Running,
    /// Automatic dispatch is paused.
    Paused,
    /// All required nodes completed successfully.
    Completed,
    /// Plan ended after an unrecoverable failure.
    Failed,
    /// Plan was explicitly stopped.
    Cancelled,
}

/// Identifies the separate structural parent relation of a plan node.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanNodeParent {
    /// Node is structurally under a session branch.
    Branch(EntityId),
    /// Node is structurally under another plan node.
    Node(EntityId),
}

/// Describes the durable result class a plan node must publish.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ExpectedResult {
    /// Stable provider-neutral result kind.
    pub kind: String,
    /// Whether at least one semantic result handle is required.
    pub requires_handle: bool,
}

/// Selects an upstream provider and model for one plan node.
///
/// Provider credentials are deliberately excluded. The provider and model IDs
/// are resolved by upstream runtime machinery at execution.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelAssignment {
    /// Upstream provider identity.
    pub provider_id: String,
    /// Upstream provider-scoped model identity.
    pub model_id: String,
    /// Optional advertised thinking-level identity.
    pub thinking_level: Option<String>,
}

/// Describes one durable planned unit located in a session tree.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanNode {
    /// Stable plan-node identity.
    pub id: EntityId,
    /// Branch or parent plan node that owns this structural child.
    pub parent: Option<PlanNodeParent>,
    /// Human-readable work title.
    pub title: String,
    /// Focused work instruction.
    pub purpose: String,
    /// Required result contract for dependency fulfillment.
    #[serde(default)]
    pub expected_result: Option<ExpectedResult>,
    /// Exact semantic handles limiting or informing the work.
    #[serde(default)]
    pub semantic_scope: Vec<String>,
    /// Current durable scheduling state.
    pub status: PlanNodeStatus,
    /// Executing branch once the planned work has started.
    pub branch: Option<EntityId>,
    /// Explicit result that can fulfill dependent work.
    pub result: Option<EntityId>,
    /// Replacement node identity when this work was explicitly superseded.
    #[serde(default)]
    pub superseded_by: Option<EntityId>,
    /// Explicit safe reason when the node was abandoned.
    #[serde(default)]
    pub abandon_reason: Option<PlanAbandonReason>,
    /// Exact failure evidence for the most recent failed attempt.
    #[serde(default)]
    pub last_failure: Option<PlanEvidenceReference>,
    /// Independent execution configuration for this work.
    pub assignment: Option<ModelAssignment>,
    /// Active worker ownership when this node is running.
    #[serde(default)]
    pub claim: Option<WorkerClaim>,
    /// Last worker claim generation, retained across release/recovery.
    #[serde(default)]
    pub claim_generation: u64,
}

/// Claims one coordinator slot for a specific runtime owner.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CoordinatorClaim {
    /// Stable claim identity.
    pub id: EntityId,
    /// Shared cancellable operation owned by this coordinator, when available.
    #[serde(default)]
    pub operation: Option<OperationId>,
    /// Runtime owner identity, never a provider credential.
    pub owner: String,
    /// Monotonic plan-local claim generation.
    pub generation: u64,
}

/// Claims one plan node and its isolated execution branch.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WorkerClaim {
    /// Stable claim identity.
    pub id: EntityId,
    /// Node whose execution is claimed.
    pub node: EntityId,
    /// Branch exclusively owned by this worker while active.
    pub branch: EntityId,
    /// Runtime owner identity, never a provider credential.
    pub owner: String,
    /// Monotonic node-local claim generation.
    pub generation: u64,
    /// Safe node assignment captured when the worker was claimed.
    #[serde(default)]
    pub assignment: Option<ModelAssignment>,
}

/// Declares that one node requires another node's explicit durable result.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanDependency {
    /// Node whose result is required.
    pub prerequisite: EntityId,
    /// Node that must wait for the result.
    pub dependent: EntityId,
    /// Exact result handle recorded when the prerequisite completes.
    pub fulfilled_by: Option<EntityId>,
}

/// Holds plan nodes and a separate acyclic dependency graph.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Plan {
    /// Stable plan identity.
    pub id: EntityId,
    /// Owning session identity.
    #[serde(default)]
    pub session: Option<EntityId>,
    /// Exact conversation location where the plan was created.
    #[serde(default)]
    pub anchor: Option<crate::ContinuationAnchor>,
    /// Durable plan goal.
    #[serde(default)]
    pub goal: String,
    /// Mutation revision, independent of activity sequence.
    #[serde(default)]
    pub revision: u64,
    /// Plan lifecycle.
    #[serde(default)]
    pub status: PlanStatus,
    /// Root session branch from which the plan was created.
    pub root_branch: EntityId,
    /// Durable plan nodes in stable insertion order.
    pub nodes: Vec<PlanNode>,
    /// Dependency edges, separate from structural parent relationships.
    pub dependencies: Vec<PlanDependency>,
    /// Current node summaries stored as independent semantic objects.
    #[serde(default)]
    pub summaries: Vec<PlanNodeSummary>,
    /// Exact evidence associations for nodes and gates.
    #[serde(default)]
    pub evidence: Vec<PlanEvidenceAssociation>,
    /// Independent typed requirements constraining node entry/completion.
    #[serde(default)]
    pub gates: Vec<PlanGate>,
    /// Branch allowed to coordinate automatic plan advancement.
    pub coordinator: Option<EntityId>,
    /// Stable node identity where the coordinator is currently focused.
    pub coordinator_cursor: Option<EntityId>,
    /// The one active auto-advancing runtime claim, if any.
    #[serde(default)]
    pub coordinator_claim: Option<CoordinatorClaim>,
    /// Last coordinator claim generation, retained across release/recovery.
    #[serde(default)]
    pub coordinator_generation: u64,
    /// Plan node identities at the top of the structural plan tree.
    #[serde(default)]
    pub root_nodes: Vec<EntityId>,
    /// Immutable inspection history of earlier plan revisions.
    #[serde(default)]
    pub history: Vec<PlanRevision>,
}

/// Retains the durable scheduling graph at one plan revision.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanRevision {
    /// Monotonic plan mutation revision.
    pub revision: u64,
    /// Goal at this revision.
    pub goal: String,
    /// Lifecycle at this revision.
    pub status: PlanStatus,
    /// Structured nodes at this revision.
    pub nodes: Vec<PlanNode>,
    /// Explicit dependency edges at this revision.
    pub dependencies: Vec<PlanDependency>,
    /// Summary records visible at this mutation revision.
    #[serde(default)]
    pub summaries: Vec<PlanNodeSummary>,
    /// Evidence associations visible at this mutation revision.
    #[serde(default)]
    pub evidence: Vec<PlanEvidenceAssociation>,
    /// Typed gates visible at this mutation revision.
    #[serde(default)]
    pub gates: Vec<PlanGate>,
    /// Safe reason for this revision when it was a replan.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Read-only execution attempt reconstructed from immutable plan history.
///
/// # Public members
/// - `branch`: Branch used for this node attempt.
/// - `claim_generation`: Monotonic node-local attempt generation.
/// - `claim`: Worker claim associated with the attempt, when retained.
/// - `status`: Latest recorded lifecycle outcome for the attempt.
/// - `result`: Accepted result reference, when the attempt completed.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanAttemptView {
    /// Branch used for this node attempt.
    pub branch: EntityId,
    /// Monotonic node-local attempt generation.
    pub claim_generation: u64,
    /// Worker claim associated with the attempt, when retained.
    pub claim: Option<EntityId>,
    /// Latest recorded lifecycle outcome for the attempt.
    pub status: PlanNodeStatus,
    /// Accepted result reference, when the attempt completed.
    pub result: Option<EntityId>,
}

/// Reports invalid plan graph operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanError {
    /// The plan node identity already exists.
    DuplicateNode,
    /// A referenced plan node does not exist.
    MissingNode,
    /// A prerequisite edge would create a dependency cycle.
    DependencyCycle,
    /// A node cannot depend on itself.
    SelfDependency,
    /// A result does not match the prerequisite's durable result.
    ResultMismatch,
    /// A plan node is not in a state that permits the requested operation.
    InvalidTransition,
    /// Plan is terminal and cannot accept lifecycle changes.
    TerminalPlan,
    /// Plan revision has no representable successor.
    RevisionExhausted,
    /// Structural parent relations contain a cycle.
    StructuralCycle,
}

#[path = "planning/plan_coordinator.rs"]
mod plan_coordinator;
#[path = "planning/plan_definition.rs"]
mod plan_definition;
#[path = "planning/plan_history.rs"]
mod plan_history;
#[path = "planning/plan_readiness.rs"]
mod plan_readiness;
#[path = "planning/plan_requirement_types.rs"]
mod plan_requirement_types;
#[path = "planning/plan_requirements.rs"]
mod plan_requirements;
#[path = "planning/plan_workers.rs"]
mod plan_workers;
/// Defines independently revisioned semantic representations of plan state.
pub mod semantic;

pub use plan_requirement_types::*;
