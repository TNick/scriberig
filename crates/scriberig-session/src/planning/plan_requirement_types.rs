//! Defines summaries, evidence, and typed plan gate records.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Identifies a typed exact semantic source used by plan knowledge.
///
/// # Public members
/// - `entity`: Stable source entity identity.
/// - `revision`: Exact source revision when the source is revisioned.
/// - `snapshot`: Exact source snapshot when the source is snapshot-bound.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanSemanticReference {
    /// Stable source entity identity.
    pub entity: EntityId,
    /// Exact source revision when the source is revisioned.
    pub revision: Option<RevisionId>,
    /// Exact source snapshot when the source is snapshot-bound.
    pub snapshot: Option<SnapshotId>,
}

/// Identifies the semantic class of one referenced evidence object.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub enum PlanEvidenceKind {
    /// Durable successful or failed branch result.
    BranchResult,
    /// Validation result with a profile and pass/fail attributes.
    ValidationResult,
    /// Resolved or unresolved semantic decision.
    Decision,
    /// Human approval with policy and role attributes.
    HumanApproval,
    /// Candidate, migration, or workspace object.
    Candidate,
    /// Artifact or externally observed object.
    Artifact,
    /// Structured execution failure result.
    Failure,
    /// Another registered semantic evidence category.
    Other(String),
}

/// Identifies one exact evidence object and its safe typed attributes.
///
/// # Public members
/// - `kind`: Semantic source class.
/// - `reference`: Exact entity, revision, and snapshot reference.
/// - `attributes`: Safe bounded source facts used by typed gate rules.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanEvidenceReference {
    /// Semantic source class.
    pub kind: PlanEvidenceKind,
    /// Exact entity, revision, and snapshot reference.
    pub reference: PlanSemanticReference,
    /// Safe bounded source facts used by typed gate rules.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

/// Identifies whether evidence supports, contradicts, or informs an owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanEvidenceRole {
    /// Evidence supports the node or gate requirement.
    Supports,
    /// Evidence contradicts the node or gate requirement.
    Contradicts,
    /// Evidence is retained for inspection without affecting evaluation.
    Informational,
}

/// Identifies the node or gate to which an evidence association belongs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanEvidenceOwner {
    /// Evidence associated directly with a plan node.
    Node(EntityId),
    /// Evidence associated with one independent plan gate.
    Gate(EntityId),
}

/// Associates one exact evidence reference with a node or gate.
///
/// # Public members
/// - `id`: Stable identity of this evidence association.
/// - `owner`: Plan node or gate receiving the evidence.
/// - `role`: Supports, contradicts, or informational role.
/// - `evidence`: Exact semantic evidence reference.
/// - `note`: Optional safe explanatory text.
/// - `actor`: Optional ScribeRig actor identity for provenance.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanEvidenceAssociation {
    /// Stable identity of this evidence association.
    pub id: EntityId,
    /// Plan node or gate receiving the evidence.
    pub owner: PlanEvidenceOwner,
    /// Supports, contradicts, or informational role.
    pub role: PlanEvidenceRole,
    /// Exact semantic evidence reference.
    pub evidence: PlanEvidenceReference,
    /// Optional safe explanatory text.
    pub note: Option<String>,
    /// Optional ScribeRig actor identity for provenance.
    pub actor: Option<EntityId>,
}

/// Links a concise current summary to one plan node.
///
/// # Public members
/// - `id`: Stable summary identity.
/// - `node`: Node summarized by this record.
/// - `text`: Current concise progress or understanding.
/// - `sources`: Exact source revisions/snapshots used to derive the summary.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanNodeSummary {
    /// Stable summary identity.
    pub id: EntityId,
    /// Node summarized by this record.
    pub node: EntityId,
    /// Current concise progress or understanding.
    pub text: String,
    /// Exact source revisions/snapshots used to derive the summary.
    pub sources: Vec<PlanSemanticReference>,
}

/// Selects when one typed gate is required.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanGatePhase {
    /// Requirement must pass before dispatch/readiness.
    Entry,
    /// Requirement must pass before completion.
    Completion,
}

/// Defines a small, queryable semantic condition for one plan gate.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanGateRequirement {
    /// Requires supporting evidence of the requested kind.
    EvidencePresent { kind: PlanEvidenceKind },
    /// Requires a passing validation for an optional named profile.
    ValidationPassed { profile: Option<String> },
    /// Requires one exact decision to be resolved.
    DecisionResolved { decision: EntityId },
    /// Requires approval under the specified policy and optional role.
    HumanApproval {
        policy: String,
        role: Option<String>,
    },
    /// Requires an exact semantic reference to satisfy a typed predicate.
    SemanticCondition {
        reference: PlanSemanticReference,
        predicate: PlanSemanticPredicate,
    },
    /// Requires one structural child to complete successfully.
    RequiredChildCompleted { child: EntityId },
}

/// Defines a safe predicate supported by a semantic-condition gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanSemanticPredicate {
    /// The exact referenced semantic entity is present as supporting evidence.
    EntityExists,
    /// The exact referenced revision is present as supporting evidence.
    ExactRevision,
}

/// Records an explicit, inspectable authorization to waive a gate.
///
/// # Public members
/// - `reason`: Nonempty safe explanation for the waiver.
/// - `actor`: Optional actor authorizing the waiver.
/// - `operation`: Optional operation identity for provenance.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanGateWaiver {
    /// Nonempty safe explanation for the waiver.
    pub reason: String,
    /// Optional actor authorizing the waiver.
    pub actor: Option<EntityId>,
    /// Optional operation identity for provenance.
    pub operation: Option<scriberig_core::OperationId>,
}

/// Represents one independently identified plan requirement.
///
/// # Public members
/// - `id`: Stable gate identity.
/// - `owner`: Plan node whose lifecycle is constrained.
/// - `phase`: Entry or completion requirement.
/// - `requirement`: Typed requirement with no executable expression language.
/// - `waiver`: Optional explicit waiver retained as canonical state.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanGate {
    /// Stable gate identity.
    pub id: EntityId,
    /// Plan node whose lifecycle is constrained.
    pub owner: EntityId,
    /// Entry or completion requirement.
    pub phase: PlanGatePhase,
    /// Typed requirement with no executable expression language.
    pub requirement: PlanGateRequirement,
    /// Optional explicit waiver retained as canonical state.
    pub waiver: Option<PlanGateWaiver>,
}

/// Represents the derived result of evaluating one gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanGateStatus {
    /// Required supporting evidence has not yet arrived.
    Pending,
    /// Exact supporting evidence satisfies the requirement.
    Satisfied,
    /// Exact contradicting or failed evidence blocks the requirement.
    Failed,
    /// An explicit canonical waiver satisfies the requirement.
    Waived,
}

/// Holds a gate result and the exact evidence that supports that result.
///
/// # Public members
/// - `gate`: Stable gate identity evaluated.
/// - `status`: Derived status using the single plan lifecycle vocabulary.
/// - `supporting_evidence`: Exact evidence references used by evaluation.
/// - `reason`: Optional bounded explanation for pending or failed state.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanGateEvaluation {
    /// Stable gate identity evaluated.
    pub gate: EntityId,
    /// Derived status using the single plan lifecycle vocabulary.
    pub status: PlanGateStatus,
    /// Exact evidence references used by evaluation.
    pub supporting_evidence: Vec<PlanEvidenceReference>,
    /// Optional bounded explanation for pending or failed state.
    pub reason: Option<String>,
}

/// Describes why a plan node was explicitly abandoned.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlanAbandonReason {
    /// Work was intentionally ignored by an authorized plan operation.
    Ignored,
    /// Work was replaced by an explicit superseding node.
    Superseded,
    /// Work was cancelled by an explicit policy decision.
    Cancelled,
    /// Work became obsolete under an explicit semantic change.
    Obsolete,
}
