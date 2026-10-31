//! Defines application requests for plan evolution and semantic mutations.

use super::EntityId;
/// Requests one ordinary work item on a running plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanNextStepRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
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

/// Describes one structural child proposed by a plan decomposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanChildDraft {
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

/// Requests structural decomposition of one plan node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanDecomposeRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Stable node identity to decompose.
    pub parent: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
    /// Proposed structural children.
    pub children: Vec<PlanChildDraft>,
    /// Whether every new child is required for parent completion.
    pub required: bool,
}

/// Describes one typed semantic mutation to an existing plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanSemanticMutation {
    /// Replaces one node's concise current summary.
    UpdateSummary {
        /// Node receiving the summary.
        node: EntityId,
        /// Bounded human-readable progress summary.
        text: String,
        /// Exact semantic source revisions and snapshots.
        sources: Vec<scriberig_session::PlanSemanticReference>,
    },
    /// Associates exact evidence with a node or gate.
    AttachEvidence {
        /// Plan node or gate receiving the evidence.
        owner: scriberig_session::PlanEvidenceOwner,
        /// Whether the evidence supports, contradicts, or informs the owner.
        role: scriberig_session::PlanEvidenceRole,
        /// Exact semantic evidence reference.
        evidence: scriberig_session::PlanEvidenceReference,
        /// Optional bounded safe explanation.
        note: Option<String>,
    },
    /// Adds a typed gate to a node.
    AddGate {
        /// Node constrained by this gate.
        owner: EntityId,
        /// Entry or completion phase.
        phase: scriberig_session::PlanGatePhase,
        /// Typed, non-executable requirement.
        requirement: scriberig_session::PlanGateRequirement,
    },
    /// Waives one gate while retaining its definition and evidence.
    WaiveGate {
        /// Gate identity to waive.
        gate: EntityId,
        /// Safe reason retained with the waiver.
        reason: String,
    },
}

/// Requests one optimistic typed semantic plan mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanSemanticMutationRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
    /// Typed mutation to apply.
    pub mutation: PlanSemanticMutation,
}
