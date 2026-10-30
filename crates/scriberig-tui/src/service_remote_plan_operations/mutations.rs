//! Sends plan evolution and semantic mutations through the protocol client.

use super::super::{ApplicationError, RemoteAdapter, service_remote_mapping};
use scriberig_app::{
    PlanDecomposeRequest, PlanNextStepRequest, PlanRetryRequest, PlanSemanticMutation,
    PlanSemanticMutationRequest,
};
use scriberig_protocol::{
    SessionPlanChildDraft, SessionPlanDecomposeRequest, SessionPlanNextStepRequest,
    SessionPlanRetryRequest, SessionPlanSemanticMutation, SessionPlanSemanticMutationRequest,
    SessionReference, SessionSemanticReference,
};

/// Retries a failed node through the shared protocol operation.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Plan, node, and observed plan revision.
///
/// The authoritative plan restored into the session domain.
/// # Returns
/// The updated plan projection.
/// # Errors
/// Returns a sanitized transport, protocol, or identity error.
pub(crate) fn retry<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &PlanRetryRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let response = adapter
        .client
        .retry_plan_node(SessionPlanRetryRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            plan: request.plan.to_string(),
            node: request.node.to_string(),
            expected_revision: request.expected_revision,
        })
        .map_err(super::super::remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Adds ordinary next-step work through the shared protocol operation.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Observed plan revision and proposed work item.
///
/// The authoritative plan restored into the session domain.
/// # Returns
/// The updated plan projection.
/// # Errors
/// Returns a sanitized transport, protocol, or identity error.
pub(crate) fn next_step<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &PlanNextStepRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let response = adapter
        .client
        .add_plan_next_step(SessionPlanNextStepRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            plan: request.plan.to_string(),
            expected_revision: request.expected_revision,
            title: request.title.clone(),
            purpose: request.purpose.clone(),
            expected_result: request.expected_result.as_ref().map(|result| {
                scriberig_protocol::SessionExpectedResult {
                    kind: result.kind.clone(),
                    requires_handle: result.requires_handle,
                }
            }),
            semantic_scope: request.semantic_scope.clone(),
            assignment: request.assignment.as_ref().map(|assignment| {
                scriberig_protocol::SessionModelAssignment {
                    provider_id: assignment.provider_id.clone(),
                    model_id: assignment.model_id.clone(),
                    thinking_level: assignment.thinking_level.clone(),
                }
            }),
        })
        .map_err(super::super::remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Decomposes a node through the shared protocol operation.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Parent, children, requirement policy, and observed revision.
///
/// The authoritative plan restored into the session domain.
/// # Returns
/// The updated plan projection.
/// # Errors
/// Returns a sanitized transport, protocol, or identity error.
pub(crate) fn decompose<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &PlanDecomposeRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let response = adapter
        .client
        .decompose_plan_node(SessionPlanDecomposeRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            plan: request.plan.to_string(),
            parent: request.parent.to_string(),
            expected_revision: request.expected_revision,
            children: request
                .children
                .iter()
                .map(|child| SessionPlanChildDraft {
                    title: child.title.clone(),
                    purpose: child.purpose.clone(),
                    expected_result: child.expected_result.as_ref().map(|result| {
                        scriberig_protocol::SessionExpectedResult {
                            kind: result.kind.clone(),
                            requires_handle: result.requires_handle,
                        }
                    }),
                    semantic_scope: child.semantic_scope.clone(),
                    assignment: child.assignment.as_ref().map(|assignment| {
                        scriberig_protocol::SessionModelAssignment {
                            provider_id: assignment.provider_id.clone(),
                            model_id: assignment.model_id.clone(),
                            thinking_level: assignment.thinking_level.clone(),
                        }
                    }),
                })
                .collect(),
            required: request.required,
        })
        .map_err(super::super::remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Applies a typed semantic mutation through the shared protocol operation.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Exact plan revision and typed mutation.
///
/// The authoritative plan restored into the session domain.
/// # Returns
/// The updated plan projection.
/// # Errors
/// Returns a sanitized transport, protocol, or identity error.
pub(crate) fn semantic_mutation<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &PlanSemanticMutationRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let response = adapter
        .client
        .mutate_plan_semantics(SessionPlanSemanticMutationRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            plan: request.plan.to_string(),
            expected_revision: request.expected_revision,
            mutation: protocol_mutation(&request.mutation),
        })
        .map_err(super::super::remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Converts one application mutation without changing its exact references.
///
/// - `mutation`: Typed shared application mutation.
///
/// The equivalent protocol mutation.
/// # Returns
/// A typed transport value retaining all stable identities and provenance inputs.
fn protocol_mutation(mutation: &PlanSemanticMutation) -> SessionPlanSemanticMutation {
    match mutation {
        PlanSemanticMutation::UpdateSummary {
            node,
            text,
            sources,
        } => SessionPlanSemanticMutation::UpdateSummary {
            node: node.to_string(),
            text: text.clone(),
            sources: sources.iter().map(protocol_reference).collect(),
        },
        PlanSemanticMutation::AttachEvidence {
            owner,
            role,
            evidence,
            note,
        } => SessionPlanSemanticMutation::AttachEvidence {
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
                reference: protocol_reference(&evidence.reference),
                attributes: evidence.attributes.clone(),
            },
            note: note.clone(),
        },
        PlanSemanticMutation::AddGate {
            owner,
            phase,
            requirement,
        } => SessionPlanSemanticMutation::AddGate {
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
        PlanSemanticMutation::WaiveGate { gate, reason } => {
            SessionPlanSemanticMutation::WaiveGate {
                gate: gate.to_string(),
                reason: reason.clone(),
            }
        }
    }
}

/// Converts an exact semantic reference to protocol string identities.
///
/// - `reference`: Entity, revision, and snapshot selected by the caller.
///
/// The matching wire reference.
/// # Returns
/// A protocol value preserving optional exact context.
fn protocol_reference(
    reference: &scriberig_session::PlanSemanticReference,
) -> SessionSemanticReference {
    SessionSemanticReference {
        entity: reference.entity.to_string(),
        revision: reference.revision.map(|value| value.to_string()),
        snapshot: reference.snapshot.map(|value| value.to_string()),
    }
}

/// Converts one evidence kind to its protocol enum.
///
/// - `kind`: Shared provider-neutral evidence category.
///
/// The equivalent transport evidence category.
/// # Returns
/// A typed protocol value retaining the `Other` payload when present.
fn protocol_evidence_kind(
    kind: &scriberig_session::PlanEvidenceKind,
) -> scriberig_protocol::SessionPlanEvidenceKind {
    match kind {
        scriberig_session::PlanEvidenceKind::BranchResult => {
            scriberig_protocol::SessionPlanEvidenceKind::BranchResult
        }
        scriberig_session::PlanEvidenceKind::ValidationResult => {
            scriberig_protocol::SessionPlanEvidenceKind::ValidationResult
        }
        scriberig_session::PlanEvidenceKind::Decision => {
            scriberig_protocol::SessionPlanEvidenceKind::Decision
        }
        scriberig_session::PlanEvidenceKind::HumanApproval => {
            scriberig_protocol::SessionPlanEvidenceKind::HumanApproval
        }
        scriberig_session::PlanEvidenceKind::Candidate => {
            scriberig_protocol::SessionPlanEvidenceKind::Candidate
        }
        scriberig_session::PlanEvidenceKind::Artifact => {
            scriberig_protocol::SessionPlanEvidenceKind::Artifact
        }
        scriberig_session::PlanEvidenceKind::Failure => {
            scriberig_protocol::SessionPlanEvidenceKind::Failure
        }
        scriberig_session::PlanEvidenceKind::Other(value) => {
            scriberig_protocol::SessionPlanEvidenceKind::Other(value.clone())
        }
    }
}

/// Converts one typed gate requirement to protocol form.
///
/// - `requirement`: Shared non-executable requirement.
///
/// The equivalent wire requirement.
/// # Returns
/// A typed protocol requirement with exact references preserved.
fn protocol_gate_requirement(
    requirement: &scriberig_session::PlanGateRequirement,
) -> scriberig_protocol::SessionPlanGateRequirement {
    use scriberig_protocol::SessionPlanGateRequirement as Wire;
    use scriberig_session::PlanGateRequirement as Shared;
    match requirement {
        Shared::EvidencePresent { kind } => Wire::EvidencePresent {
            kind: protocol_evidence_kind(kind),
        },
        Shared::ValidationPassed { profile } => Wire::ValidationPassed {
            profile: profile.clone(),
        },
        Shared::DecisionResolved { decision } => Wire::DecisionResolved {
            decision: decision.to_string(),
        },
        Shared::HumanApproval { policy, role } => Wire::HumanApproval {
            policy: policy.clone(),
            role: role.clone(),
        },
        Shared::SemanticCondition {
            reference,
            predicate,
        } => Wire::SemanticCondition {
            reference: protocol_reference(reference),
            predicate: match predicate {
                scriberig_session::PlanSemanticPredicate::EntityExists => {
                    scriberig_protocol::SessionPlanSemanticPredicate::EntityExists
                }
                scriberig_session::PlanSemanticPredicate::ExactRevision => {
                    scriberig_protocol::SessionPlanSemanticPredicate::ExactRevision
                }
            },
        },
        Shared::RequiredChildCompleted { child } => Wire::RequiredChildCompleted {
            child: child.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserves exact gate identity and waiver reason in the remote mutation.
    #[test]
    fn gate_waiver_keeps_typed_identity_and_reason() {
        let gate = scriberig_core::EntityId::from_bytes([91; 16]);
        let mutation = protocol_mutation(&PlanSemanticMutation::WaiveGate {
            gate,
            reason: "reviewed exception".to_owned(),
        });

        assert_eq!(
            mutation,
            SessionPlanSemanticMutation::WaiveGate {
                gate: gate.to_string(),
                reason: "reviewed exception".to_owned(),
            }
        );
    }
}
