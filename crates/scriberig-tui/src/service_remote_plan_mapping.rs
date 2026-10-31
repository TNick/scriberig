//! Restores complete safe plan state from remote session responses.

use super::{ApplicationError, parse_entity, parse_plan_parent};

#[path = "service_remote_plan_mapping/semantics.rs"]
mod semantics;

/// Restores a durable plan and its retained revisions from the protocol.
///
/// - `value`: Safe remote plan projection.
///
/// Local session-domain plan state with stable identities restored.
///
/// Returns invalid-request when any stable identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn from_response(
    value: scriberig_protocol::SessionPlan,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let coordinator_generation = value
        .coordinator_claim
        .as_ref()
        .map_or(0, |claim| claim.generation);
    Ok(scriberig_session::Plan {
        id: parse_entity(&value.id)?,
        source_snapshot: semantics::snapshot(value.snapshot.as_deref())?,
        session: None,
        anchor: value.anchor.map(anchor).transpose()?,
        goal: value.goal,
        revision: value.revision,
        status: plan_status(value.status),
        root_branch: parse_entity(&value.root_branch)?,
        nodes: value
            .nodes
            .into_iter()
            .map(node)
            .collect::<Result<Vec<_>, _>>()?,
        dependencies: value
            .dependencies
            .iter()
            .map(dependency)
            .collect::<Result<Vec<_>, _>>()?,
        summaries: value
            .summaries
            .into_iter()
            .map(semantics::summary)
            .collect::<Result<Vec<_>, _>>()?,
        evidence: value
            .evidence
            .into_iter()
            .map(semantics::evidence)
            .collect::<Result<Vec<_>, _>>()?,
        gates: value
            .gates
            .into_iter()
            .map(semantics::gate)
            .collect::<Result<Vec<_>, _>>()?,
        coordinator: value.coordinator.as_deref().map(parse_entity).transpose()?,
        coordinator_cursor: value
            .coordinator_cursor
            .as_deref()
            .map(parse_entity)
            .transpose()?,
        coordinator_claim: value
            .coordinator_claim
            .map(|claim| {
                Ok(scriberig_session::CoordinatorClaim {
                    id: parse_entity(&claim.id)?,
                    operation: claim
                        .operation
                        .map(scriberig_core::OperationId::new)
                        .transpose()
                        .map_err(|error| ApplicationError::InvalidRequest {
                            operation: "session plan".to_owned(),
                            message: format!("invalid coordinator operation: {error}"),
                        })?,
                    owner: claim.owner,
                    generation: claim.generation,
                })
            })
            .transpose()?,
        coordinator_generation,
        root_nodes: value
            .root_nodes
            .iter()
            .map(|identity| parse_entity(identity))
            .collect::<Result<Vec<_>, _>>()?,
        history: value
            .history
            .into_iter()
            .map(revision)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Restores one protocol history entry.
///
/// - `value`: Immutable remote plan revision.
///
/// Local historical projection.
///
/// Returns invalid-request for malformed node or dependency identities.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn revision(
    value: scriberig_protocol::SessionPlanRevision,
) -> Result<scriberig_session::PlanRevision, ApplicationError> {
    Ok(scriberig_session::PlanRevision {
        revision: value.revision,
        goal: value.goal,
        status: plan_status(value.status),
        nodes: value
            .nodes
            .into_iter()
            .map(node)
            .collect::<Result<Vec<_>, _>>()?,
        dependencies: value
            .dependencies
            .iter()
            .map(dependency)
            .collect::<Result<Vec<_>, _>>()?,
        summaries: value
            .summaries
            .into_iter()
            .map(semantics::summary)
            .collect::<Result<Vec<_>, _>>()?,
        evidence: value
            .evidence
            .into_iter()
            .map(semantics::evidence)
            .collect::<Result<Vec<_>, _>>()?,
        gates: value
            .gates
            .into_iter()
            .map(semantics::gate)
            .collect::<Result<Vec<_>, _>>()?,
        reason: value.reason,
    })
}

/// Restores one remote planned node and its active worker claim.
///
/// - `value`: Safe remote node projection.
///
/// Local plan node with assignment and result identity restored.
///
/// Returns invalid-request when an identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn node(
    value: scriberig_protocol::SessionPlanNode,
) -> Result<scriberig_session::PlanNode, ApplicationError> {
    let claim_generation = value.claim.as_ref().map_or(0, |claim| claim.generation);
    Ok(scriberig_session::PlanNode {
        id: parse_entity(&value.id)?,
        parent: parse_plan_parent(&value.parent)?,
        title: value.title,
        purpose: value.purpose,
        expected_result: value
            .expected_result
            .map(|expected| scriberig_session::ExpectedResult {
                kind: expected.kind,
                requires_handle: expected.requires_handle,
            }),
        semantic_scope: value.semantic_scope,
        status: match value.status {
            scriberig_protocol::SessionPlanNodeStatus::Planned => {
                scriberig_session::PlanNodeStatus::Planned
            }
            scriberig_protocol::SessionPlanNodeStatus::Ready => {
                scriberig_session::PlanNodeStatus::Ready
            }
            scriberig_protocol::SessionPlanNodeStatus::Running => {
                scriberig_session::PlanNodeStatus::Running
            }
            scriberig_protocol::SessionPlanNodeStatus::Completed => {
                scriberig_session::PlanNodeStatus::Completed
            }
            scriberig_protocol::SessionPlanNodeStatus::Failed => {
                scriberig_session::PlanNodeStatus::Failed
            }
            scriberig_protocol::SessionPlanNodeStatus::Abandoned => {
                scriberig_session::PlanNodeStatus::Abandoned
            }
        },
        branch: value.branch.as_deref().map(parse_entity).transpose()?,
        result: value.result.as_deref().map(parse_entity).transpose()?,
        superseded_by: value
            .superseded_by
            .as_deref()
            .map(parse_entity)
            .transpose()?,
        abandon_reason: semantics::abandon_reason(value.abandon_reason),
        last_failure: value
            .last_failure
            .map(semantics::evidence_reference)
            .transpose()?,
        assignment: value.assignment.map(assignment),
        claim: value
            .claim
            .map(|claim| {
                Ok(scriberig_session::WorkerClaim {
                    id: parse_entity(&claim.id)?,
                    node: parse_entity(&claim.node)?,
                    branch: parse_entity(&claim.branch)?,
                    owner: claim.owner,
                    generation: claim.generation,
                    assignment: claim.assignment.map(assignment),
                })
            })
            .transpose()?,
        claim_generation,
    })
}

/// Restores one safe profile selection.
///
/// - `value`: Credential-free remote assignment.
///
/// Local provider-neutral assignment.
/// # Returns
/// The operation result.
fn assignment(
    value: scriberig_protocol::SessionModelAssignment,
) -> scriberig_session::ModelAssignment {
    scriberig_session::ModelAssignment {
        provider_id: value.provider_id,
        model_id: value.model_id,
        thinking_level: value.thinking_level,
    }
}

/// Restores one remote result dependency.
///
/// - `value`: Safe remote dependency edge.
///
/// Local result-linked edge.
///
/// Returns invalid-request when an edge identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn dependency(
    value: &scriberig_protocol::SessionPlanDependency,
) -> Result<scriberig_session::PlanDependency, ApplicationError> {
    Ok(scriberig_session::PlanDependency {
        prerequisite: parse_entity(&value.prerequisite)?,
        dependent: parse_entity(&value.dependent)?,
        fulfilled_by: value
            .fulfilled_by
            .as_deref()
            .map(parse_entity)
            .transpose()?,
    })
}

/// Restores an exact continuation anchor.
///
/// - `value`: Remote stable continuation location.
///
/// Local exact session anchor.
///
/// Returns invalid-request when an identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn anchor(
    value: scriberig_protocol::SessionContinuationAnchor,
) -> Result<scriberig_session::ContinuationAnchor, ApplicationError> {
    Ok(match value {
        scriberig_protocol::SessionContinuationAnchor::BranchHead { branch } => {
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: parse_entity(&branch)?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::AssistantOutput { branch, turn } => {
            scriberig_session::ContinuationAnchor::AssistantOutput {
                branch: parse_entity(&branch)?,
                turn: parse_entity(&turn)?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::HumanInput { branch, turn } => {
            scriberig_session::ContinuationAnchor::HumanInput {
                branch: parse_entity(&branch)?,
                turn: parse_entity(&turn)?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::BranchResult { branch, result } => {
            scriberig_session::ContinuationAnchor::BranchResult {
                branch: parse_entity(&branch)?,
                result: parse_entity(&result)?,
            }
        }
    })
}

/// Converts a local continuation anchor to its public protocol representation.
///
/// - `value`: Stable local continuation location.
///
/// Equivalent credential-free protocol anchor.
/// # Returns
/// The operation result.
pub(crate) fn protocol_anchor(
    value: scriberig_session::ContinuationAnchor,
) -> scriberig_protocol::SessionContinuationAnchor {
    match value {
        scriberig_session::ContinuationAnchor::BranchHead { branch } => {
            scriberig_protocol::SessionContinuationAnchor::BranchHead {
                branch: branch.to_string(),
            }
        }
        scriberig_session::ContinuationAnchor::AssistantOutput { branch, turn } => {
            scriberig_protocol::SessionContinuationAnchor::AssistantOutput {
                branch: branch.to_string(),
                turn: turn.to_string(),
            }
        }
        scriberig_session::ContinuationAnchor::HumanInput { branch, turn } => {
            scriberig_protocol::SessionContinuationAnchor::HumanInput {
                branch: branch.to_string(),
                turn: turn.to_string(),
            }
        }
        scriberig_session::ContinuationAnchor::BranchResult { branch, result } => {
            scriberig_protocol::SessionContinuationAnchor::BranchResult {
                branch: branch.to_string(),
                result: result.to_string(),
            }
        }
    }
}

/// Converts one remote plan lifecycle state.
///
/// - `value`: Protocol lifecycle variant.
///
/// Shared session-domain state.
/// # Returns
/// The operation result.
fn plan_status(value: scriberig_protocol::SessionPlanStatus) -> scriberig_session::PlanStatus {
    match value {
        scriberig_protocol::SessionPlanStatus::Draft => scriberig_session::PlanStatus::Draft,
        scriberig_protocol::SessionPlanStatus::Planned => scriberig_session::PlanStatus::Planned,
        scriberig_protocol::SessionPlanStatus::Running => scriberig_session::PlanStatus::Running,
        scriberig_protocol::SessionPlanStatus::Paused => scriberig_session::PlanStatus::Paused,
        scriberig_protocol::SessionPlanStatus::Completed => {
            scriberig_session::PlanStatus::Completed
        }
        scriberig_protocol::SessionPlanStatus::Failed => scriberig_session::PlanStatus::Failed,
        scriberig_protocol::SessionPlanStatus::Cancelled => {
            scriberig_session::PlanStatus::Cancelled
        }
    }
}
