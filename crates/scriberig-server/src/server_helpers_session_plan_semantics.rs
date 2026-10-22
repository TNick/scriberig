//! Converts semantic plan gates, evidence, and summaries into safe DTOs.

use scriberig_protocol::{
    SessionPlanAbandonReason, SessionPlanEvidenceAssociation, SessionPlanEvidenceKind,
    SessionPlanEvidenceOwner, SessionPlanEvidenceReference, SessionPlanEvidenceRole,
    SessionPlanGate, SessionPlanGateCounts, SessionPlanGatePhase, SessionPlanGateRequirement,
    SessionPlanGateWaiver, SessionPlanSemanticPredicate, SessionPlanStatusCounts,
    SessionPlanSummary, SessionSemanticReference,
};
use scriberig_session::{
    PlanAbandonReason, PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner,
    PlanEvidenceReference, PlanEvidenceRole, PlanGate, PlanGateCounts, PlanGatePhase,
    PlanGateRequirement, PlanGateWaiver, PlanNodeStatusCounts, PlanNodeSummary,
    PlanSemanticPredicate, PlanSemanticReference,
};

/// Converts direct node status totals to protocol values.
///
/// # Parameters
/// - `value`: Shared direct node status counts.
///
/// A protocol-safe count record.
pub(super) fn status_counts(value: PlanNodeStatusCounts) -> SessionPlanStatusCounts {
    SessionPlanStatusCounts {
        planned: value.planned,
        ready: value.ready,
        running: value.running,
        completed: value.completed,
        failed: value.failed,
        abandoned: value.abandoned,
    }
}

/// Converts gate evaluation counts to protocol values.
///
/// # Parameters
/// - `value`: Shared gate-state counts.
///
/// A protocol-safe gate count record.
pub(super) fn gate_counts(value: PlanGateCounts) -> SessionPlanGateCounts {
    SessionPlanGateCounts {
        pending: value.pending,
        satisfied: value.satisfied,
        failed: value.failed,
        waived: value.waived,
    }
}

/// Converts one canonical gate definition.
///
/// # Parameters
/// - `value`: Shared typed gate and waiver.
///
/// A protocol gate without physical storage information.
pub(super) fn gate(value: &PlanGate) -> SessionPlanGate {
    SessionPlanGate {
        id: value.id.to_string(),
        owner: value.owner.to_string(),
        phase: match value.phase {
            PlanGatePhase::Entry => SessionPlanGatePhase::Entry,
            PlanGatePhase::Completion => SessionPlanGatePhase::Completion,
        },
        requirement: match &value.requirement {
            PlanGateRequirement::EvidencePresent { kind } => {
                SessionPlanGateRequirement::EvidencePresent {
                    kind: evidence_kind(kind),
                }
            }
            PlanGateRequirement::ValidationPassed { profile } => {
                SessionPlanGateRequirement::ValidationPassed {
                    profile: profile.clone(),
                }
            }
            PlanGateRequirement::DecisionResolved { decision } => {
                SessionPlanGateRequirement::DecisionResolved {
                    decision: decision.to_string(),
                }
            }
            PlanGateRequirement::HumanApproval { policy, role } => {
                SessionPlanGateRequirement::HumanApproval {
                    policy: policy.clone(),
                    role: role.clone(),
                }
            }
            PlanGateRequirement::SemanticCondition {
                reference,
                predicate,
            } => SessionPlanGateRequirement::SemanticCondition {
                reference: semantic_reference(reference),
                predicate: match predicate {
                    PlanSemanticPredicate::EntityExists => {
                        SessionPlanSemanticPredicate::EntityExists
                    }
                    PlanSemanticPredicate::ExactRevision => {
                        SessionPlanSemanticPredicate::ExactRevision
                    }
                },
            },
            PlanGateRequirement::RequiredChildCompleted { child } => {
                SessionPlanGateRequirement::RequiredChildCompleted {
                    child: child.to_string(),
                }
            }
        },
        waiver: value.waiver.as_ref().map(waiver),
    }
}

/// Converts one canonical evidence association.
///
/// # Parameters
/// - `value`: Shared exact evidence association.
///
/// A protocol-safe evidence record.
pub(super) fn evidence(value: &PlanEvidenceAssociation) -> SessionPlanEvidenceAssociation {
    SessionPlanEvidenceAssociation {
        id: value.id.to_string(),
        owner: match value.owner {
            PlanEvidenceOwner::Node(node) => SessionPlanEvidenceOwner::Node(node.to_string()),
            PlanEvidenceOwner::Gate(gate) => SessionPlanEvidenceOwner::Gate(gate.to_string()),
        },
        role: match value.role {
            PlanEvidenceRole::Supports => SessionPlanEvidenceRole::Supports,
            PlanEvidenceRole::Contradicts => SessionPlanEvidenceRole::Contradicts,
            PlanEvidenceRole::Informational => SessionPlanEvidenceRole::Informational,
        },
        evidence: evidence_reference(&value.evidence),
        note: value.note.clone(),
        actor: value.actor.map(|actor| actor.to_string()),
    }
}

/// Converts a typed evidence reference without copying source payloads.
///
/// # Parameters
/// - `value`: Exact semantic reference and safe typed attributes.
///
/// A protocol evidence reference.
pub(super) fn evidence_reference(value: &PlanEvidenceReference) -> SessionPlanEvidenceReference {
    SessionPlanEvidenceReference {
        kind: evidence_kind(&value.kind),
        reference: semantic_reference(&value.reference),
        attributes: value.attributes.clone(),
    }
}

/// Converts one concise summary and its exact source references.
///
/// # Parameters
/// - `value`: Shared current node summary.
///
/// A protocol summary record.
pub(super) fn summary(value: &PlanNodeSummary) -> SessionPlanSummary {
    SessionPlanSummary {
        id: value.id.to_string(),
        node: value.node.to_string(),
        text: value.text.clone(),
        sources: value.sources.iter().map(semantic_reference).collect(),
    }
}

/// Converts a semantic source reference to stable wire identities.
///
/// # Parameters
/// - `value`: Shared entity/revision/snapshot reference.
///
/// A protocol-safe reference.
fn semantic_reference(value: &PlanSemanticReference) -> SessionSemanticReference {
    SessionSemanticReference {
        entity: value.entity.to_string(),
        revision: value.revision.map(|id| id.to_string()),
        snapshot: value.snapshot.map(|id| id.to_string()),
    }
}

/// Converts a semantic evidence kind to the transport vocabulary.
///
/// # Parameters
/// - `value`: Shared registered evidence kind.
///
/// The matching wire evidence kind.
fn evidence_kind(value: &PlanEvidenceKind) -> SessionPlanEvidenceKind {
    match value {
        PlanEvidenceKind::BranchResult => SessionPlanEvidenceKind::BranchResult,
        PlanEvidenceKind::ValidationResult => SessionPlanEvidenceKind::ValidationResult,
        PlanEvidenceKind::Decision => SessionPlanEvidenceKind::Decision,
        PlanEvidenceKind::HumanApproval => SessionPlanEvidenceKind::HumanApproval,
        PlanEvidenceKind::Candidate => SessionPlanEvidenceKind::Candidate,
        PlanEvidenceKind::Artifact => SessionPlanEvidenceKind::Artifact,
        PlanEvidenceKind::Failure => SessionPlanEvidenceKind::Failure,
        PlanEvidenceKind::Other(kind) => SessionPlanEvidenceKind::Other(kind.clone()),
    }
}

/// Converts waiver provenance to a stable protocol record.
///
/// # Parameters
/// - `value`: Shared waiver reason and actor/operation identities.
///
/// A protocol waiver record.
fn waiver(value: &PlanGateWaiver) -> SessionPlanGateWaiver {
    SessionPlanGateWaiver {
        reason: value.reason.clone(),
        actor: value.actor.map(|actor| actor.to_string()),
        operation: value.operation.as_ref().map(ToString::to_string),
    }
}

/// Converts an explicit abandonment reason to its transport vocabulary.
///
/// # Parameters
/// - `value`: Shared typed node abandonment reason.
///
/// The matching wire value.
pub(super) fn abandon_reason(value: PlanAbandonReason) -> SessionPlanAbandonReason {
    match value {
        PlanAbandonReason::Ignored => SessionPlanAbandonReason::Ignored,
        PlanAbandonReason::Superseded => SessionPlanAbandonReason::Superseded,
        PlanAbandonReason::Cancelled => SessionPlanAbandonReason::Cancelled,
        PlanAbandonReason::Obsolete => SessionPlanAbandonReason::Obsolete,
    }
}
