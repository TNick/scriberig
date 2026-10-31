//! Converts typed plan requirement mutations from protocol to application values.

use std::str::FromStr;

use scriberig_app::{ApplicationError, PlanSemanticMutation};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_session::{
    PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceReference, PlanEvidenceRole, PlanGatePhase,
    PlanGateRequirement, PlanSemanticPredicate, PlanSemanticReference,
};

/// Converts one typed protocol semantic mutation.
///
/// - `value`: Protocol mutation with string-encoded identities.
///
/// Shared application mutation.
/// # Returns
/// The converted mutation.
/// # Errors
/// Returns an invalid-request error for malformed identities.
pub(super) fn semantic_mutation(
    value: scriberig_protocol::SessionPlanSemanticMutation,
) -> Result<PlanSemanticMutation, ApplicationError> {
    Ok(match value {
        scriberig_protocol::SessionPlanSemanticMutation::UpdateSummary {
            node,
            text,
            sources,
        } => PlanSemanticMutation::UpdateSummary {
            node: parse_entity(&node)?,
            text,
            sources: sources
                .into_iter()
                .map(semantic_reference)
                .collect::<Result<Vec<_>, _>>()?,
        },
        scriberig_protocol::SessionPlanSemanticMutation::AttachEvidence {
            owner,
            role,
            evidence,
            note,
        } => PlanSemanticMutation::AttachEvidence {
            owner: match owner {
                scriberig_protocol::SessionPlanEvidenceOwner::Node(node) => {
                    PlanEvidenceOwner::Node(parse_entity(&node)?)
                }
                scriberig_protocol::SessionPlanEvidenceOwner::Gate(gate) => {
                    PlanEvidenceOwner::Gate(parse_entity(&gate)?)
                }
            },
            role: match role {
                scriberig_protocol::SessionPlanEvidenceRole::Supports => PlanEvidenceRole::Supports,
                scriberig_protocol::SessionPlanEvidenceRole::Contradicts => {
                    PlanEvidenceRole::Contradicts
                }
                scriberig_protocol::SessionPlanEvidenceRole::Informational => {
                    PlanEvidenceRole::Informational
                }
            },
            evidence: PlanEvidenceReference {
                kind: evidence_kind(evidence.kind),
                reference: semantic_reference(evidence.reference)?,
                attributes: evidence.attributes,
            },
            note,
        },
        scriberig_protocol::SessionPlanSemanticMutation::AddGate {
            owner,
            phase,
            requirement,
        } => PlanSemanticMutation::AddGate {
            owner: parse_entity(&owner)?,
            phase: match phase {
                scriberig_protocol::SessionPlanGatePhase::Entry => PlanGatePhase::Entry,
                scriberig_protocol::SessionPlanGatePhase::Completion => PlanGatePhase::Completion,
            },
            requirement: gate_requirement(requirement)?,
        },
        scriberig_protocol::SessionPlanSemanticMutation::WaiveGate { gate, reason } => {
            PlanSemanticMutation::WaiveGate {
                gate: parse_entity(&gate)?,
                reason,
            }
        }
    })
}

/// Converts one exact protocol source reference.
///
/// - `value`: Entity, revision, and snapshot identities from the request.
///
/// Shared application semantic reference.
/// # Returns
/// The converted reference.
/// # Errors
/// Returns an invalid-request error for malformed identities.
fn semantic_reference(
    value: scriberig_protocol::SessionSemanticReference,
) -> Result<PlanSemanticReference, ApplicationError> {
    Ok(PlanSemanticReference {
        entity: parse_entity(&value.entity)?,
        revision: value
            .revision
            .map(|revision| RevisionId::from_str(&revision))
            .transpose()
            .map_err(invalid_identity)?,
        snapshot: value
            .snapshot
            .map(|snapshot| SnapshotId::from_str(&snapshot))
            .transpose()
            .map_err(invalid_identity)?,
    })
}

/// Converts one protocol evidence kind.
///
/// - `value`: Typed protocol evidence category.
///
/// Shared session evidence category.
/// # Returns
/// The converted evidence kind.
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

/// Converts one protocol gate requirement.
///
/// - `value`: Typed, non-executable protocol gate requirement.
///
/// Shared session gate requirement.
/// # Returns
/// The converted requirement.
/// # Errors
/// Returns an invalid-request error for malformed identities.
fn gate_requirement(
    value: scriberig_protocol::SessionPlanGateRequirement,
) -> Result<PlanGateRequirement, ApplicationError> {
    Ok(match value {
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
            reference,
            predicate,
        } => PlanGateRequirement::SemanticCondition {
            reference: semantic_reference(reference)?,
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
    })
}

/// Parses one stable semantic entity identity from protocol text.
///
/// - `value`: Protocol string form of an entity identity.
///
/// Parsed entity identity.
/// # Returns
/// The parsed identity.
/// # Errors
/// Returns invalid-request when the string is malformed.
fn parse_entity(value: &str) -> Result<EntityId, ApplicationError> {
    EntityId::from_str(value).map_err(invalid_identity)
}

/// Creates a structured invalid-identity error.
///
/// - `error`: Parsing failure from a typed identity.
///
/// An application invalid-request error.
/// # Returns
/// The structured invalid-identity error.
fn invalid_identity(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "plan semantic mutation".to_owned(),
        message: error.to_string(),
    }
}
