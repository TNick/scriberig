//! Restores typed plan semantics from protocol values.

use std::str::FromStr;

use super::super::{ApplicationError, parse_entity, parse_snapshot};
use scriberig_core::{OperationId, RevisionId};
use scriberig_protocol as protocol;
use scriberig_session as session;

/// Restores one node summary with its exact semantic sources.
///
/// - `value`: Safe protocol summary record.
///
/// A session-domain summary with validated identities.
///
/// Returns an internal error when any remote identity is malformed.
/// # Returns
/// The restored summary.
/// # Errors
/// Returns an error when a referenced identity is invalid.
pub(super) fn summary(
    value: protocol::SessionPlanSummary,
) -> Result<session::PlanNodeSummary, ApplicationError> {
    Ok(session::PlanNodeSummary {
        id: parse_entity(&value.id)?,
        node: parse_entity(&value.node)?,
        text: value.text,
        sources: value
            .sources
            .into_iter()
            .map(|reference| semantic_reference(&reference))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Restores one exact evidence association.
///
/// - `value`: Safe protocol association.
///
/// A session-domain association with validated owner and actor identities.
///
/// Returns an internal error when any remote identity is malformed.
/// # Returns
/// The restored evidence association.
/// # Errors
/// Returns an error when a referenced identity is invalid.
pub(super) fn evidence(
    value: protocol::SessionPlanEvidenceAssociation,
) -> Result<session::PlanEvidenceAssociation, ApplicationError> {
    Ok(session::PlanEvidenceAssociation {
        id: parse_entity(&value.id)?,
        owner: match value.owner {
            protocol::SessionPlanEvidenceOwner::Node(node) => {
                session::PlanEvidenceOwner::Node(parse_entity(&node)?)
            }
            protocol::SessionPlanEvidenceOwner::Gate(gate) => {
                session::PlanEvidenceOwner::Gate(parse_entity(&gate)?)
            }
        },
        role: match value.role {
            protocol::SessionPlanEvidenceRole::Supports => session::PlanEvidenceRole::Supports,
            protocol::SessionPlanEvidenceRole::Contradicts => {
                session::PlanEvidenceRole::Contradicts
            }
            protocol::SessionPlanEvidenceRole::Informational => {
                session::PlanEvidenceRole::Informational
            }
        },
        evidence: evidence_reference(value.evidence)?,
        note: value.note,
        actor: value.actor.as_deref().map(parse_entity).transpose()?,
    })
}

/// Restores one typed gate and optional waiver provenance.
///
/// - `value`: Safe protocol gate record.
///
/// A session-domain gate with validated identities.
///
/// Returns an internal error when any remote identity is malformed.
/// # Returns
/// The restored gate.
/// # Errors
/// Returns an error when a referenced identity is invalid.
pub(super) fn gate(
    value: protocol::SessionPlanGate,
) -> Result<session::PlanGate, ApplicationError> {
    Ok(session::PlanGate {
        id: parse_entity(&value.id)?,
        owner: parse_entity(&value.owner)?,
        phase: match value.phase {
            protocol::SessionPlanGatePhase::Entry => session::PlanGatePhase::Entry,
            protocol::SessionPlanGatePhase::Completion => session::PlanGatePhase::Completion,
        },
        requirement: match value.requirement {
            protocol::SessionPlanGateRequirement::EvidencePresent { kind } => {
                session::PlanGateRequirement::EvidencePresent {
                    kind: evidence_kind(kind),
                }
            }
            protocol::SessionPlanGateRequirement::ValidationPassed { profile } => {
                session::PlanGateRequirement::ValidationPassed { profile }
            }
            protocol::SessionPlanGateRequirement::DecisionResolved { decision } => {
                session::PlanGateRequirement::DecisionResolved {
                    decision: parse_entity(&decision)?,
                }
            }
            protocol::SessionPlanGateRequirement::HumanApproval { policy, role } => {
                session::PlanGateRequirement::HumanApproval { policy, role }
            }
            protocol::SessionPlanGateRequirement::SemanticCondition {
                reference,
                predicate,
            } => session::PlanGateRequirement::SemanticCondition {
                reference: semantic_reference(&reference)?,
                predicate: match predicate {
                    protocol::SessionPlanSemanticPredicate::EntityExists => {
                        session::PlanSemanticPredicate::EntityExists
                    }
                    protocol::SessionPlanSemanticPredicate::ExactRevision => {
                        session::PlanSemanticPredicate::ExactRevision
                    }
                },
            },
            protocol::SessionPlanGateRequirement::RequiredChildCompleted { child } => {
                session::PlanGateRequirement::RequiredChildCompleted {
                    child: parse_entity(&child)?,
                }
            }
        },
        waiver: value.waiver.map(gate_waiver).transpose()?,
    })
}

/// Restores one exact evidence reference without source payloads.
///
/// - `value`: Safe protocol evidence.
///
/// A session-domain evidence reference.
///
/// Returns an internal error when its semantic source identity is malformed.
/// # Returns
/// The restored reference.
/// # Errors
/// Returns an error when a referenced identity is invalid.
pub(super) fn evidence_reference(
    value: protocol::SessionPlanEvidenceReference,
) -> Result<session::PlanEvidenceReference, ApplicationError> {
    Ok(session::PlanEvidenceReference {
        kind: evidence_kind(value.kind),
        reference: semantic_reference(&value.reference)?,
        attributes: value.attributes,
    })
}

/// Restores an exact semantic source reference.
///
/// - `value`: Protocol entity, revision, and snapshot identities.
///
/// A validated session-domain source reference.
///
/// Returns an internal error when an identity is malformed.
/// # Returns
/// The restored source reference.
/// # Errors
/// Returns an error when a referenced identity is invalid.
fn semantic_reference(
    value: &protocol::SessionSemanticReference,
) -> Result<session::PlanSemanticReference, ApplicationError> {
    Ok(session::PlanSemanticReference {
        entity: parse_entity(&value.entity)?,
        revision: value.revision.as_deref().map(parse_revision).transpose()?,
        snapshot: value.snapshot.as_deref().map(parse_snapshot).transpose()?,
    })
}

/// Restores the stable vocabulary for semantic evidence kinds.
///
/// - `value`: Protocol evidence kind.
///
/// The equivalent session-domain evidence kind.
/// # Returns
/// The restored evidence kind.
fn evidence_kind(value: protocol::SessionPlanEvidenceKind) -> session::PlanEvidenceKind {
    match value {
        protocol::SessionPlanEvidenceKind::BranchResult => session::PlanEvidenceKind::BranchResult,
        protocol::SessionPlanEvidenceKind::ValidationResult => {
            session::PlanEvidenceKind::ValidationResult
        }
        protocol::SessionPlanEvidenceKind::Decision => session::PlanEvidenceKind::Decision,
        protocol::SessionPlanEvidenceKind::HumanApproval => {
            session::PlanEvidenceKind::HumanApproval
        }
        protocol::SessionPlanEvidenceKind::Candidate => session::PlanEvidenceKind::Candidate,
        protocol::SessionPlanEvidenceKind::Artifact => session::PlanEvidenceKind::Artifact,
        protocol::SessionPlanEvidenceKind::Failure => session::PlanEvidenceKind::Failure,
        protocol::SessionPlanEvidenceKind::Other(kind) => session::PlanEvidenceKind::Other(kind),
    }
}

/// Restores waiver reason and actor/operation provenance.
///
/// - `value`: Protocol waiver.
///
/// A session-domain gate waiver.
///
/// Returns an internal error when the actor or operation identity is malformed.
/// # Returns
/// The restored waiver.
/// # Errors
/// Returns an error when a referenced identity is invalid.
fn gate_waiver(
    value: protocol::SessionPlanGateWaiver,
) -> Result<session::PlanGateWaiver, ApplicationError> {
    Ok(session::PlanGateWaiver {
        reason: value.reason,
        actor: value.actor.as_deref().map(parse_entity).transpose()?,
        operation: value
            .operation
            .as_deref()
            .map(parse_operation)
            .transpose()?,
    })
}

/// Restores an explicit node-abandonment reason.
///
/// - `value`: Protocol abandonment reason.
///
/// The equivalent session-domain reason.
/// # Returns
/// The restored reason.
pub(super) fn abandon_reason(
    value: protocol::SessionPlanAbandonReason,
) -> session::PlanAbandonReason {
    match value {
        protocol::SessionPlanAbandonReason::Ignored => session::PlanAbandonReason::Ignored,
        protocol::SessionPlanAbandonReason::Superseded => session::PlanAbandonReason::Superseded,
        protocol::SessionPlanAbandonReason::Cancelled => session::PlanAbandonReason::Cancelled,
        protocol::SessionPlanAbandonReason::Obsolete => session::PlanAbandonReason::Obsolete,
    }
}

/// Parses an exact revision identity from the remote protocol.
///
/// - `value`: Stable revision identity text.
///
/// A validated immutable revision identity.
///
/// Returns an internal error for malformed identity text.
/// # Returns
/// The parsed revision identity.
/// # Errors
/// Returns an error when the identity is invalid.
fn parse_revision(value: &str) -> Result<RevisionId, ApplicationError> {
    RevisionId::from_str(value).map_err(|error| ApplicationError::Internal {
        message: format!("remote response contained invalid revision identity: {error}"),
    })
}

/// Parses an operation identity from waiver provenance.
///
/// - `value`: Stable operation identity text.
///
/// A validated operation identity.
///
/// Returns an internal error for malformed identity text.
/// # Returns
/// The parsed operation identity.
/// # Errors
/// Returns an error when the identity is invalid.
fn parse_operation(value: &str) -> Result<OperationId, ApplicationError> {
    OperationId::from_str(value).map_err(|error| ApplicationError::Internal {
        message: format!("remote response contained invalid operation identity: {error}"),
    })
}
