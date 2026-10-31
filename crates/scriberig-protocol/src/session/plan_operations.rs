//! Defines next-step and decomposition requests for durable plans.

use super::plan::{SessionExpectedResult, SessionModelAssignment};
use super::plan_semantics::{
    SessionPlanEvidenceOwner, SessionPlanEvidenceReference, SessionPlanEvidenceRole,
    SessionPlanGatePhase, SessionPlanGateRequirement, SessionSemanticReference,
};
use crate::SessionReference;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Adds ordinary work to a running plan at an observed revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanNextStepRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Plan revision observed by the client.
    pub expected_revision: u64,
    /// Human-readable work title.
    pub title: String,
    /// Focused work instruction.
    pub purpose: String,
    /// Expected provider-neutral result contract.
    pub expected_result: Option<SessionExpectedResult>,
    /// Semantic handles selected for this work.
    pub semantic_scope: Vec<String>,
    /// Optional safe model-library assignment.
    #[serde(default)]
    pub assignment: Option<SessionModelAssignment>,
}

/// One proposed structural child in a plan decomposition.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanChildDraft {
    /// Human-readable work title.
    pub title: String,
    /// Focused work instruction.
    pub purpose: String,
    /// Expected provider-neutral result contract.
    pub expected_result: Option<SessionExpectedResult>,
    /// Semantic handles selected for this work.
    pub semantic_scope: Vec<String>,
    /// Optional safe model-library assignment.
    #[serde(default)]
    pub assignment: Option<SessionModelAssignment>,
}

/// Decomposes one node into structural children at an observed plan revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanDecomposeRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Stable node identity to decompose.
    pub parent: String,
    /// Plan revision observed by the client.
    pub expected_revision: u64,
    /// Proposed structural children.
    pub children: Vec<SessionPlanChildDraft>,
    /// Whether every new child is required for parent completion.
    pub required: bool,
}

/// Describes a typed mutation of plan summaries, evidence, and gates.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionPlanSemanticMutation {
    /// Replaces one node's current concise summary.
    UpdateSummary {
        /// Node receiving the summary.
        node: String,
        /// Bounded human-readable progress summary.
        text: String,
        /// Exact source revisions and snapshots.
        sources: Vec<SessionSemanticReference>,
    },
    /// Associates exact evidence with a node or gate.
    AttachEvidence {
        /// Plan node or gate receiving the evidence.
        owner: SessionPlanEvidenceOwner,
        /// Whether the evidence supports, contradicts, or informs the owner.
        role: SessionPlanEvidenceRole,
        /// Exact semantic evidence reference.
        evidence: SessionPlanEvidenceReference,
        /// Optional bounded safe explanation.
        #[serde(default)]
        note: Option<String>,
    },
    /// Adds a typed gate to a plan node.
    AddGate {
        /// Node constrained by this gate.
        owner: String,
        /// Entry or completion phase.
        phase: SessionPlanGatePhase,
        /// Typed, non-executable requirement.
        requirement: SessionPlanGateRequirement,
    },
    /// Waives one gate while retaining its definition and evidence.
    WaiveGate {
        /// Gate identity to waive.
        gate: String,
        /// Safe reason retained with the waiver.
        reason: String,
    },
}

/// Applies one typed semantic mutation at an observed plan revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanSemanticMutationRequest {
    /// Owning session identity.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Plan revision observed by the client.
    pub expected_revision: u64,
    /// Typed plan mutation to apply.
    pub mutation: SessionPlanSemanticMutation,
}
