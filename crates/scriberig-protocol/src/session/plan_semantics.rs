//! Defines protocol values for semantic plan gates, evidence, and summaries.

use super::plan::{
    SessionContinuationAnchor, SessionCoordinatorClaim, SessionPlanDependency, SessionPlanNode,
    SessionPlanStatus,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
/// Counts direct plan nodes by their one lifecycle vocabulary.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanStatusCounts {
    /// Nodes waiting for dependencies or gates.
    pub planned: u32,
    /// Nodes ready for a worker claim.
    pub ready: u32,
    /// Nodes with active worker claims.
    pub running: u32,
    /// Nodes completed with accepted results.
    pub completed: u32,
    /// Nodes with a failed attempt.
    pub failed: u32,
    /// Explicitly abandoned nodes.
    pub abandoned: u32,
}

/// Counts evaluated requirements in one lifecycle phase.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanGateCounts {
    /// Gates awaiting evidence or child completion.
    pub pending: u32,
    /// Gates satisfied by canonical state.
    pub satisfied: u32,
    /// Gates blocked by contradicting or failed evidence.
    pub failed: u32,
    /// Gates explicitly waived with provenance.
    pub waived: u32,
}

/// Describes why a plan node was abandoned.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanAbandonReason {
    /// Work was intentionally ignored.
    Ignored,
    /// Work was replaced by an explicit superseding node.
    Superseded,
    /// Work was cancelled by explicit policy.
    Cancelled,
    /// Work became obsolete after semantic change.
    Obsolete,
}

/// Identifies an exact semantic object or snapshot used by plan knowledge.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionSemanticReference {
    /// Stable semantic entity identity.
    pub entity: String,
    /// Exact immutable revision identity, when available.
    #[serde(default)]
    pub revision: Option<String>,
    /// Exact universal snapshot identity, when available.
    #[serde(default)]
    pub snapshot: Option<String>,
}

/// Identifies the typed source category of evidence.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanEvidenceKind {
    /// Durable successful or failed branch result.
    BranchResult,
    /// Validation result with profile and pass/fail state.
    ValidationResult,
    /// Resolved or unresolved semantic decision.
    Decision,
    /// Human approval under a named policy.
    HumanApproval,
    /// Candidate or migration proposal.
    Candidate,
    /// Artifact or external observation.
    Artifact,
    /// Structured execution failure result.
    Failure,
    /// Another registered evidence class.
    Other(String),
}

/// Carries an exact evidence reference with bounded typed facts.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanEvidenceReference {
    /// Semantic evidence category.
    pub kind: SessionPlanEvidenceKind,
    /// Exact entity/revision/snapshot identity.
    pub reference: SessionSemanticReference,
    /// Safe bounded attributes used by typed requirements.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

/// Identifies whether evidence supports, contradicts, or informs an owner.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanEvidenceRole {
    /// Evidence supports the associated node or gate.
    Supports,
    /// Evidence contradicts the associated node or gate.
    Contradicts,
    /// Evidence is retained for inspection only.
    Informational,
}

/// Identifies a plan node or gate that owns an evidence association.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanEvidenceOwner {
    /// Evidence belongs to a plan node.
    Node(String),
    /// Evidence belongs to a typed gate.
    Gate(String),
}

/// Associates exact evidence with a node or gate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanEvidenceAssociation {
    /// Stable evidence-association identity.
    pub id: String,
    /// Node or gate receiving this association.
    pub owner: SessionPlanEvidenceOwner,
    /// How evidence affects interpretation.
    pub role: SessionPlanEvidenceRole,
    /// Exact semantic source and safe facts.
    pub evidence: SessionPlanEvidenceReference,
    /// Optional bounded explanatory note.
    #[serde(default)]
    pub note: Option<String>,
    /// Optional ScribeRig actor identity.
    #[serde(default)]
    pub actor: Option<String>,
}

/// Selects when a typed gate is required.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanGatePhase {
    /// Requirement must pass before dispatch.
    Entry,
    /// Requirement must pass before completion.
    Completion,
}

/// Selects a supported, non-executable semantic condition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanSemanticPredicate {
    /// Exact entity is present as supporting evidence.
    EntityExists,
    /// Exact referenced revision is present as supporting evidence.
    ExactRevision,
}

/// Describes one supported plan-gate requirement.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanGateRequirement {
    /// Requires supporting evidence of the requested kind.
    EvidencePresent {
        /// Evidence category required by this gate.
        kind: SessionPlanEvidenceKind,
    },
    /// Requires passing validation for an optional profile.
    ValidationPassed {
        /// Required validation profile, when constrained.
        profile: Option<String>,
    },
    /// Requires one exact decision to be resolved.
    DecisionResolved {
        /// Stable decision entity identity.
        decision: String,
    },
    /// Requires approval under a policy and optional role.
    HumanApproval {
        /// Approval policy identity.
        policy: String,
        /// Required approver role, when constrained.
        role: Option<String>,
    },
    /// Requires an exact semantic reference to satisfy a typed predicate.
    SemanticCondition {
        /// Entity/revision/snapshot reference to check.
        reference: SessionSemanticReference,
        /// Supported non-executable predicate.
        predicate: SessionPlanSemanticPredicate,
    },
    /// Requires one structural child to complete.
    RequiredChildCompleted {
        /// Stable child node identity.
        child: String,
    },
}

/// Retains explicit waiver reason and actor provenance.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanGateWaiver {
    /// Safe reason for waiving the requirement.
    pub reason: String,
    /// Actor authorizing the waiver.
    #[serde(default)]
    pub actor: Option<String>,
    /// Shared operation that recorded the waiver.
    #[serde(default)]
    pub operation: Option<String>,
}

/// Projects one independent typed lifecycle gate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanGate {
    /// Stable gate identity.
    pub id: String,
    /// Node whose lifecycle is constrained.
    pub owner: String,
    /// Entry or completion phase.
    pub phase: SessionPlanGatePhase,
    /// Typed requirement without executable expressions.
    pub requirement: SessionPlanGateRequirement,
    /// Explicit retained waiver provenance.
    #[serde(default)]
    pub waiver: Option<SessionPlanGateWaiver>,
}

/// Projects a concise node summary with exact source references.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanSummary {
    /// Stable summary identity.
    pub id: String,
    /// Node summarized by the record.
    pub node: String,
    /// Concise current progress or understanding.
    pub text: String,
    /// Exact source revisions/snapshots used to derive the summary.
    #[serde(default)]
    pub sources: Vec<SessionSemanticReference>,
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
    /// Summary records visible at this historical revision.
    #[serde(default)]
    pub summaries: Vec<SessionPlanSummary>,
    /// Typed requirements visible at this historical revision.
    #[serde(default)]
    pub gates: Vec<SessionPlanGate>,
    /// Exact evidence associations visible at this historical revision.
    #[serde(default)]
    pub evidence: Vec<SessionPlanEvidenceAssociation>,
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
    /// Exact immutable Plan revision identity.
    #[serde(default)]
    pub exact_revision: Option<String>,
    /// Exact universal snapshot represented by this projection.
    #[serde(default)]
    pub snapshot: Option<String>,
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
    /// Structural root nodes in stable membership order.
    #[serde(default)]
    pub root_nodes: Vec<String>,
    /// Direct status counts across all plan nodes.
    #[serde(default)]
    pub status_counts: SessionPlanStatusCounts,
    /// Current typed node lifecycle requirements.
    #[serde(default)]
    pub gates: Vec<SessionPlanGate>,
    /// Current exact evidence associations.
    #[serde(default)]
    pub evidence: Vec<SessionPlanEvidenceAssociation>,
    /// Current concise node summaries and exact sources.
    #[serde(default)]
    pub summaries: Vec<SessionPlanSummary>,
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
