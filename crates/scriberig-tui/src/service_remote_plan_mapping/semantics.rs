//! Restores semantic plan details from provider-neutral protocol projections.

use scriberig_core::{OperationId, RevisionId, SnapshotId};
use scriberig_session::{
    PlanAbandonReason, PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner,
    PlanEvidenceReference, PlanEvidenceRole, PlanGate, PlanGatePhase, PlanGateRequirement,
    PlanGateWaiver, PlanNodeSummary, PlanSemanticPredicate, PlanSemanticReference,
};

use super::{ApplicationError, parse_entity};

/// Creates a bounded protocol mapping error for semantic plan details.
///
/// - `message`: Safe parser diagnostic.
///
/// An invalid-request application error.
/// # Returns
/// The requested validation error.
fn invalid(message: String) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "remote plan projection".to_owned(),
        message,
    }
}

/// Parses one exact optional revision identity from the remote projection.
///
/// - `value`: Protocol revision identity when present.
///
/// The typed optional revision identity.
///
/// Returns invalid-request when the revision encoding is malformed.
/// # Returns
/// The typed optional revision identity.
/// # Errors
/// Returns an error when the revision encoding is malformed.
pub(super) fn revision(value: Option<&str>) -> Result<Option<RevisionId>, ApplicationError> {
    value
        .map(str::parse)
        .transpose()
        .map_err(|error| invalid(format!("invalid revision identity: {error}")))
}

/// Parses one exact optional snapshot identity from the remote projection.
///
/// - `value`: Protocol snapshot identity when present.
///
/// The typed optional snapshot identity.
///
/// Returns invalid-request when the snapshot encoding is malformed.
/// # Returns
/// The typed optional snapshot identity.
/// # Errors
/// Returns an error when the snapshot encoding is malformed.
pub(super) fn snapshot(value: Option<&str>) -> Result<Option<SnapshotId>, ApplicationError> {
    value
        .map(str::parse)
        .transpose()
        .map_err(|error| invalid(format!("invalid snapshot identity: {error}")))
}

/// Restores one exact source reference carried by a plan projection.
///
/// - `value`: Protocol reference with optional revision and snapshot IDs.
///
/// The shared exact semantic reference.
///
/// Returns invalid-request when an identity is malformed.
/// # Returns
/// The typed semantic reference.
/// # Errors
/// Returns an error when an entity, revision, or snapshot identity is invalid.
pub(super) fn reference(
    value: &scriberig_protocol::SessionSemanticReference,
) -> Result<PlanSemanticReference, ApplicationError> {
    Ok(PlanSemanticReference {
        entity: parse_entity(&value.entity)?,
        revision: revision(value.revision.as_deref())?,
        snapshot: snapshot(value.snapshot.as_deref())?,
    })
}

/// Restores one exact evidence reference and its bounded facts.
///
/// - `value`: Protocol-safe evidence reference.
///
/// The shared semantic evidence reference.
///
/// Returns invalid-request when an identity is malformed.
/// # Returns
/// The typed evidence reference.
/// # Errors
/// Returns an error when an entity, revision, or snapshot identity is invalid.
pub(super) fn evidence_reference(
    value: scriberig_protocol::SessionPlanEvidenceReference,
) -> Result<PlanEvidenceReference, ApplicationError> {
    Ok(PlanEvidenceReference {
        kind: evidence_kind(value.kind),
        reference: reference(&value.reference)?,
        attributes: value.attributes,
    })
}

/// Restores a typed gate and its optional waiver provenance.
///
/// - `value`: Protocol-safe gate projection.
///
/// The shared semantic gate.
///
/// Returns invalid-request when any stable identity is malformed.
/// # Returns
/// The typed gate.
/// # Errors
/// Returns an error when an entity, revision, or snapshot identity is invalid.
pub(super) fn gate(
    value: scriberig_protocol::SessionPlanGate,
) -> Result<PlanGate, ApplicationError> {
    let requirement = match value.requirement {
        scriberig_protocol::SessionPlanGateRequirement::EvidencePresent { kind } => {
            PlanGateRequirement::EvidencePresent {
                kind: evidence_kind(kind),
            }
        }
        scriberig_protocol::SessionPlanGateRequirement::ValidationPassed { profile } => {
            PlanGateRequirement::ValidationPassed { profile }
        }
        scriberig_protocol::SessionPlanGateRequirement::DecisionResolved { decision } => {
            PlanGateRequirement::DecisionResolved {
                decision: parse_entity(&decision)?,
            }
        }
        scriberig_protocol::SessionPlanGateRequirement::HumanApproval { policy, role } => {
            PlanGateRequirement::HumanApproval { policy, role }
        }
        scriberig_protocol::SessionPlanGateRequirement::SemanticCondition {
            reference: source,
            predicate,
        } => PlanGateRequirement::SemanticCondition {
            reference: reference(&source)?,
            predicate: match predicate {
                scriberig_protocol::SessionPlanSemanticPredicate::EntityExists => {
                    PlanSemanticPredicate::EntityExists
                }
                scriberig_protocol::SessionPlanSemanticPredicate::ExactRevision => {
                    PlanSemanticPredicate::ExactRevision
                }
            },
        },
        scriberig_protocol::SessionPlanGateRequirement::RequiredChildCompleted { child } => {
            PlanGateRequirement::RequiredChildCompleted {
                child: parse_entity(&child)?,
            }
        }
    };
    let waiver = value
        .waiver
        .map(|waiver| {
            Ok::<_, ApplicationError>(PlanGateWaiver {
                reason: waiver.reason,
                actor: waiver.actor.as_deref().map(parse_entity).transpose()?,
                operation: waiver
                    .operation
                    .map(OperationId::new)
                    .transpose()
                    .map_err(|error| invalid(format!("invalid waiver operation: {error}")))?,
            })
        })
        .transpose()?;
    Ok(PlanGate {
        id: parse_entity(&value.id)?,
        owner: parse_entity(&value.owner)?,
        phase: match value.phase {
            scriberig_protocol::SessionPlanGatePhase::Entry => PlanGatePhase::Entry,
            scriberig_protocol::SessionPlanGatePhase::Completion => PlanGatePhase::Completion,
        },
        requirement,
        waiver,
    })
}

/// Restores one evidence association without losing ownership or provenance.
///
/// - `value`: Protocol-safe evidence association.
///
/// The shared semantic evidence association.
///
/// Returns invalid-request when a stable identity is malformed.
/// # Returns
/// The typed evidence association.
/// # Errors
/// Returns an error when an identity is malformed.
pub(super) fn evidence(
    value: scriberig_protocol::SessionPlanEvidenceAssociation,
) -> Result<PlanEvidenceAssociation, ApplicationError> {
    Ok(PlanEvidenceAssociation {
        id: parse_entity(&value.id)?,
        owner: match value.owner {
            scriberig_protocol::SessionPlanEvidenceOwner::Node(node) => {
                PlanEvidenceOwner::Node(parse_entity(&node)?)
            }
            scriberig_protocol::SessionPlanEvidenceOwner::Gate(gate) => {
                PlanEvidenceOwner::Gate(parse_entity(&gate)?)
            }
        },
        role: match value.role {
            scriberig_protocol::SessionPlanEvidenceRole::Supports => PlanEvidenceRole::Supports,
            scriberig_protocol::SessionPlanEvidenceRole::Contradicts => {
                PlanEvidenceRole::Contradicts
            }
            scriberig_protocol::SessionPlanEvidenceRole::Informational => {
                PlanEvidenceRole::Informational
            }
        },
        evidence: evidence_reference(value.evidence)?,
        note: value.note,
        actor: value.actor.as_deref().map(parse_entity).transpose()?,
    })
}

/// Restores one concise summary and all of its exact source references.
///
/// - `value`: Protocol-safe summary projection.
///
/// The shared plan summary.
///
/// Returns invalid-request when an identity is malformed.
/// # Returns
/// The typed summary.
/// # Errors
/// Returns an error when an entity, revision, or snapshot identity is invalid.
pub(super) fn summary(
    value: scriberig_protocol::SessionPlanSummary,
) -> Result<PlanNodeSummary, ApplicationError> {
    Ok(PlanNodeSummary {
        id: parse_entity(&value.id)?,
        node: parse_entity(&value.node)?,
        text: value.text,
        sources: value
            .sources
            .into_iter()
            .map(|source| reference(&source))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Restores a protocol evidence kind to its shared registered category.
///
/// - `value`: Protocol evidence category.
///
/// The corresponding shared evidence kind.
/// # Returns
/// The typed evidence kind.
fn evidence_kind(value: scriberig_protocol::SessionPlanEvidenceKind) -> PlanEvidenceKind {
    match value {
        scriberig_protocol::SessionPlanEvidenceKind::BranchResult => PlanEvidenceKind::BranchResult,
        scriberig_protocol::SessionPlanEvidenceKind::ValidationResult => {
            PlanEvidenceKind::ValidationResult
        }
        scriberig_protocol::SessionPlanEvidenceKind::Decision => PlanEvidenceKind::Decision,
        scriberig_protocol::SessionPlanEvidenceKind::HumanApproval => {
            PlanEvidenceKind::HumanApproval
        }
        scriberig_protocol::SessionPlanEvidenceKind::Candidate => PlanEvidenceKind::Candidate,
        scriberig_protocol::SessionPlanEvidenceKind::Artifact => PlanEvidenceKind::Artifact,
        scriberig_protocol::SessionPlanEvidenceKind::Failure => PlanEvidenceKind::Failure,
        scriberig_protocol::SessionPlanEvidenceKind::Other(kind) => PlanEvidenceKind::Other(kind),
    }
}

/// Restores an optional typed abandonment reason.
///
/// - `value`: Protocol reason when the node was abandoned.
///
/// The corresponding shared plan abandonment reason.
/// # Returns
/// The typed optional reason.
pub(super) fn abandon_reason(
    value: Option<scriberig_protocol::SessionPlanAbandonReason>,
) -> Option<PlanAbandonReason> {
    value.map(|reason| match reason {
        scriberig_protocol::SessionPlanAbandonReason::Ignored => PlanAbandonReason::Ignored,
        scriberig_protocol::SessionPlanAbandonReason::Superseded => PlanAbandonReason::Superseded,
        scriberig_protocol::SessionPlanAbandonReason::Cancelled => PlanAbandonReason::Cancelled,
        scriberig_protocol::SessionPlanAbandonReason::Obsolete => PlanAbandonReason::Obsolete,
    })
}
