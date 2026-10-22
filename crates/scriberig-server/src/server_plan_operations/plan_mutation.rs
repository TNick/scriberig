//! Adapts semantic plan mutations from protocol to application values.

use super::{ApplicationError, node_draft, parse_entity};

/// Converts one protocol plan mutation to the shared application request.
///
/// - `mutation`: Typed, transport-neutral semantic mutation.
///
/// # Returns
/// The typed shared application mutation.
///
/// # Errors
/// Returns an error when an identity or semantic reference is malformed.
///
/// The exhaustive wire-to-domain mapping stays together so every operation is
/// translated at the same trust boundary.
#[allow(clippy::too_many_lines)]
pub(super) fn plan_mutation(
    mutation: &scriberig_protocol::SessionPlanMutation,
) -> Result<scriberig_app::PlanMutation, ApplicationError> {
    use scriberig_protocol::SessionPlanMutation as Wire;
    use scriberig_session::{
        PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceReference, PlanEvidenceRole,
        PlanGatePhase, PlanGateRequirement, PlanSemanticPredicate, PlanSemanticReference,
    };

    let parse_semantic_reference = |reference: &scriberig_protocol::SessionSemanticReference| {
        Ok::<_, ApplicationError>(PlanSemanticReference {
            entity: parse_entity(&reference.entity)?,
            revision: reference
                .revision
                .as_deref()
                .map(str::parse::<scriberig_core::RevisionId>)
                .transpose()
                .map_err(|error| invalid_identity(error.to_string()))?,
            snapshot: reference
                .snapshot
                .as_deref()
                .map(str::parse::<scriberig_core::SnapshotId>)
                .transpose()
                .map_err(|error| invalid_identity(error.to_string()))?,
        })
    };
    let evidence_kind = |kind: &scriberig_protocol::SessionPlanEvidenceKind| match kind {
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
        scriberig_protocol::SessionPlanEvidenceKind::Other(name) => {
            PlanEvidenceKind::Other(name.clone())
        }
    };
    let requirement = |value: &scriberig_protocol::SessionPlanGateRequirement| {
        Ok::<_, ApplicationError>(match value {
            scriberig_protocol::SessionPlanGateRequirement::EvidencePresent { kind } => {
                PlanGateRequirement::EvidencePresent {
                    kind: evidence_kind(kind),
                }
            }
            scriberig_protocol::SessionPlanGateRequirement::ValidationPassed { profile } => {
                PlanGateRequirement::ValidationPassed {
                    profile: profile.clone(),
                }
            }
            scriberig_protocol::SessionPlanGateRequirement::DecisionResolved { decision } => {
                PlanGateRequirement::DecisionResolved {
                    decision: parse_entity(decision)?,
                }
            }
            scriberig_protocol::SessionPlanGateRequirement::HumanApproval { policy, role } => {
                PlanGateRequirement::HumanApproval {
                    policy: policy.clone(),
                    role: role.clone(),
                }
            }
            scriberig_protocol::SessionPlanGateRequirement::SemanticCondition {
                reference,
                predicate,
            } => PlanGateRequirement::SemanticCondition {
                reference: parse_semantic_reference(reference)?,
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
                    child: parse_entity(child)?,
                }
            }
        })
    };
    Ok(match mutation {
        Wire::AddNextStep { node } => scriberig_app::PlanMutation::AddNextStep(node_draft(node)),
        Wire::Decompose {
            parent,
            children,
            required,
        } => scriberig_app::PlanMutation::Decompose {
            parent: parse_entity(parent)?,
            children: children.iter().map(node_draft).collect(),
            required: *required,
        },
        Wire::AddGate {
            owner,
            phase,
            requirement: wire_requirement,
        } => scriberig_app::PlanMutation::AddGate {
            owner: parse_entity(owner)?,
            phase: match phase {
                scriberig_protocol::SessionPlanGatePhase::Entry => PlanGatePhase::Entry,
                scriberig_protocol::SessionPlanGatePhase::Completion => PlanGatePhase::Completion,
            },
            requirement: requirement(wire_requirement)?,
        },
        Wire::WaiveGate { gate, reason } => scriberig_app::PlanMutation::WaiveGate {
            gate: parse_entity(gate)?,
            reason: reason.clone(),
        },
        Wire::AttachEvidence {
            owner,
            role,
            evidence,
            note,
        } => scriberig_app::PlanMutation::AttachEvidence {
            owner: match owner {
                scriberig_protocol::SessionPlanEvidenceOwner::Node(node) => {
                    PlanEvidenceOwner::Node(parse_entity(node)?)
                }
                scriberig_protocol::SessionPlanEvidenceOwner::Gate(gate) => {
                    PlanEvidenceOwner::Gate(parse_entity(gate)?)
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
                kind: evidence_kind(&evidence.kind),
                reference: parse_semantic_reference(&evidence.reference)?,
                attributes: evidence.attributes.clone(),
            },
            note: note.clone(),
        },
        Wire::UpdateSummary {
            node,
            text,
            sources,
        } => scriberig_app::PlanMutation::UpdateSummary {
            node: parse_entity(node)?,
            text: text.clone(),
            sources: sources
                .iter()
                .map(parse_semantic_reference)
                .collect::<Result<Vec<_>, _>>()?,
        },
        Wire::RetryFailedNode { node } => scriberig_app::PlanMutation::RetryFailedNode {
            node: parse_entity(node)?,
        },
        Wire::RecoverFailedPlan => scriberig_app::PlanMutation::RecoverFailedPlan,
    })
}

/// Creates a structured error for a malformed semantic reference.
///
/// - `message`: Bounded identity parse diagnostic.
///
/// # Returns
/// An invalid-request error associated with plan mutation.
fn invalid_identity(message: String) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "plan mutation".to_owned(),
        message,
    }
}
