//! Routes typed semantic plan mutations through the remote protocol client.

use super::super::{remote_error, service_remote_mapping};
use super::{ApplicationError, RemoteAdapter, protocol_node_draft};
use scriberig_app::{PlanMutation, PlanMutationRequest};
use scriberig_protocol::{
    SessionPlanMutation, SessionPlanMutationRequest, SessionReference, SessionSemanticReference,
};

/// Applies a supported typed plan mutation through the protocol client.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Plan identity, expected revision, and semantic mutation.
///
/// The authoritative plan restored into the session domain.
///
/// Returns a sanitized transport, protocol, or identity error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails or the mutation is unavailable remotely.
pub fn mutate<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &PlanMutationRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let mutation = match &request.mutation {
        PlanMutation::AddNextStep(node) => SessionPlanMutation::AddNextStep {
            node: protocol_node_draft(node),
        },
        PlanMutation::Decompose {
            parent,
            children,
            required,
        } => SessionPlanMutation::Decompose {
            parent: parent.to_string(),
            children: children.iter().map(protocol_node_draft).collect(),
            required: *required,
        },
        PlanMutation::AddGate {
            owner,
            phase,
            requirement,
        } => SessionPlanMutation::AddGate {
            owner: owner.to_string(),
            phase: match phase {
                scriberig_session::PlanGatePhase::Entry => {
                    scriberig_protocol::SessionPlanGatePhase::Entry
                }
                scriberig_session::PlanGatePhase::Completion => {
                    scriberig_protocol::SessionPlanGatePhase::Completion
                }
            },
            requirement: protocol_gate_requirement(requirement),
        },
        PlanMutation::WaiveGate { gate, reason } => SessionPlanMutation::WaiveGate {
            gate: gate.to_string(),
            reason: reason.clone(),
        },
        PlanMutation::AttachEvidence {
            owner,
            role,
            evidence,
            note,
        } => SessionPlanMutation::AttachEvidence {
            owner: match owner {
                scriberig_session::PlanEvidenceOwner::Node(node) => {
                    scriberig_protocol::SessionPlanEvidenceOwner::Node(node.to_string())
                }
                scriberig_session::PlanEvidenceOwner::Gate(gate) => {
                    scriberig_protocol::SessionPlanEvidenceOwner::Gate(gate.to_string())
                }
            },
            role: match role {
                scriberig_session::PlanEvidenceRole::Supports => {
                    scriberig_protocol::SessionPlanEvidenceRole::Supports
                }
                scriberig_session::PlanEvidenceRole::Contradicts => {
                    scriberig_protocol::SessionPlanEvidenceRole::Contradicts
                }
                scriberig_session::PlanEvidenceRole::Informational => {
                    scriberig_protocol::SessionPlanEvidenceRole::Informational
                }
            },
            evidence: scriberig_protocol::SessionPlanEvidenceReference {
                kind: protocol_evidence_kind(&evidence.kind),
                reference: protocol_semantic_reference(evidence.reference),
                attributes: evidence.attributes.clone(),
            },
            note: note.clone(),
        },
        PlanMutation::UpdateSummary {
            node,
            text,
            sources,
        } => SessionPlanMutation::UpdateSummary {
            node: node.to_string(),
            text: text.clone(),
            sources: sources
                .iter()
                .copied()
                .map(protocol_semantic_reference)
                .collect(),
        },
        PlanMutation::RetryFailedNode { node } => SessionPlanMutation::RetryFailedNode {
            node: node.to_string(),
        },
        PlanMutation::RecoverFailedPlan => SessionPlanMutation::RecoverFailedPlan,
    };
    let response = adapter
        .client
        .mutate_plan(SessionPlanMutationRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            plan: request.plan.to_string(),
            expected_revision: request.expected_revision,
            mutation,
        })
        .map_err(remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Converts a typed gate requirement to its protocol representation.
///
/// - `requirement`: Shared semantic gate requirement.
///
/// The equivalent protocol requirement.
/// # Returns
/// The wire gate requirement.
fn protocol_gate_requirement(
    requirement: &scriberig_session::PlanGateRequirement,
) -> scriberig_protocol::SessionPlanGateRequirement {
    use scriberig_protocol::SessionPlanGateRequirement as Wire;
    use scriberig_session::PlanGateRequirement as Domain;
    match requirement {
        Domain::EvidencePresent { kind } => Wire::EvidencePresent {
            kind: protocol_evidence_kind(kind),
        },
        Domain::ValidationPassed { profile } => Wire::ValidationPassed {
            profile: profile.clone(),
        },
        Domain::DecisionResolved { decision } => Wire::DecisionResolved {
            decision: decision.to_string(),
        },
        Domain::HumanApproval { policy, role } => Wire::HumanApproval {
            policy: policy.clone(),
            role: role.clone(),
        },
        Domain::SemanticCondition {
            reference,
            predicate,
        } => Wire::SemanticCondition {
            reference: protocol_semantic_reference(*reference),
            predicate: match predicate {
                scriberig_session::PlanSemanticPredicate::EntityExists => {
                    scriberig_protocol::SessionPlanSemanticPredicate::EntityExists
                }
                scriberig_session::PlanSemanticPredicate::ExactRevision => {
                    scriberig_protocol::SessionPlanSemanticPredicate::ExactRevision
                }
            },
        },
        Domain::RequiredChildCompleted { child } => Wire::RequiredChildCompleted {
            child: child.to_string(),
        },
    }
}

/// Converts a semantic evidence category to its protocol value.
///
/// - `kind`: Shared evidence classification.
///
/// The equivalent protocol evidence category.
/// # Returns
/// The wire evidence kind.
fn protocol_evidence_kind(
    kind: &scriberig_session::PlanEvidenceKind,
) -> scriberig_protocol::SessionPlanEvidenceKind {
    use scriberig_protocol::SessionPlanEvidenceKind as Wire;
    use scriberig_session::PlanEvidenceKind as Domain;
    match kind {
        Domain::BranchResult => Wire::BranchResult,
        Domain::ValidationResult => Wire::ValidationResult,
        Domain::Decision => Wire::Decision,
        Domain::HumanApproval => Wire::HumanApproval,
        Domain::Candidate => Wire::Candidate,
        Domain::Artifact => Wire::Artifact,
        Domain::Failure => Wire::Failure,
        Domain::Other(name) => Wire::Other(name.clone()),
    }
}

/// Converts an exact semantic reference to its protocol value.
///
/// - `reference`: Shared entity, optional revision, and optional snapshot.
///
/// The equivalent protocol reference.
/// # Returns
/// The wire semantic reference.
fn protocol_semantic_reference(
    reference: scriberig_session::PlanSemanticReference,
) -> SessionSemanticReference {
    SessionSemanticReference {
        entity: reference.entity.to_string(),
        revision: reference.revision.map(|revision| revision.to_string()),
        snapshot: reference.snapshot.map(|snapshot| snapshot.to_string()),
    }
}
