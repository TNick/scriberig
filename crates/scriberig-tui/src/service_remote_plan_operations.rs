//! Sends shared plan operations through the remote protocol client.

use super::{ApplicationError, RemoteAdapter, service_remote_mapping};
#[path = "service_remote_plan_operations/mutations.rs"]
mod mutations;
pub(super) use mutations::{decompose, next_step, retry, semantic_mutation};
use scriberig_app::{
    PlanAssignmentRequest, PlanCreateRequest, PlanLifecycleAction, PlanLifecycleRequest,
};
use scriberig_protocol::{
    SessionModelAssignment, SessionPlanAction, SessionPlanAssignmentRequest,
    SessionPlanCreateRequest, SessionPlanLifecycleRequest, SessionPlanNodeDraft,
    SessionPlanNodeReference, SessionPlanReplanRequest, SessionReference,
};

/// Creates one durable plan through the existing client operation.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Shared application plan proposal.
///
/// The authoritative plan restored into the session domain.
///
/// Returns a sanitized transport, protocol, or identity error.
pub(super) fn create<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &PlanCreateRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let response = adapter
        .client
        .create_plan(SessionPlanCreateRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            anchor: service_remote_mapping::protocol_anchor(request.anchor),
            goal: request.goal.clone(),
            nodes: request
                .nodes
                .iter()
                .map(|node| SessionPlanNodeDraft {
                    key: node.key.clone(),
                    parent_index: node.parent_index,
                    title: node.title.clone(),
                    purpose: node.purpose.clone(),
                    expected_result: node.expected_result.as_ref().map(|result| {
                        scriberig_protocol::SessionExpectedResult {
                            kind: result.kind.clone(),
                            requires_handle: result.requires_handle,
                        }
                    }),
                    semantic_scope: node.semantic_scope.clone(),
                    assignment: node.assignment.as_ref().map(protocol_assignment),
                })
                .collect(),
            dependencies: request.dependencies.clone(),
        })
        .map_err(super::remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Applies one plan lifecycle transition through the protocol client.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Shared plan identity and observed revision.
/// - `action`: Requested lifecycle transition.
///
/// The authoritative plan restored into the session domain.
///
/// Returns a sanitized transport, protocol, or identity error.
pub(super) fn lifecycle<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &PlanLifecycleRequest,
    action: PlanLifecycleAction,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let response = adapter
        .client
        .plan_lifecycle(SessionPlanLifecycleRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            plan: request.plan.to_string(),
            expected_revision: request.expected_revision,
            action: match action {
                PlanLifecycleAction::Start => SessionPlanAction::Start,
                PlanLifecycleAction::Pause => SessionPlanAction::Pause,
                PlanLifecycleAction::Resume => SessionPlanAction::Resume,
                PlanLifecycleAction::Stop => SessionPlanAction::Stop,
                PlanLifecycleAction::Complete => SessionPlanAction::Complete,
            },
        })
        .map_err(super::remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Changes a node assignment using the shared protocol operation.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Plan, node, observed revision, and assignment.
///
/// The authoritative plan restored into the session domain.
///
/// Returns a sanitized transport, protocol, or identity error.
pub(super) fn assign<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &PlanAssignmentRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let response = adapter
        .client
        .set_plan_assignment(SessionPlanAssignmentRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            plan: request.plan.to_string(),
            node: request.node.to_string(),
            expected_revision: request.expected_revision,
            assignment: request.assignment.as_ref().map(protocol_assignment),
        })
        .map_err(super::remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Applies one complete plan revision through the protocol client.
///
/// - `adapter`: Remote adapter owning the protocol client.
/// - `request`: Complete shared application replan proposal.
///
/// The authoritative plan restored into the session domain.
///
/// Returns a sanitized transport, protocol, or identity error.
pub(super) fn replan<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
    request: &scriberig_app::PlanReplanRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    let response = adapter
        .client
        .replan(SessionPlanReplanRequest {
            session: SessionReference {
                id: request.session.to_string(),
            },
            plan: request.plan.to_string(),
            expected_revision: request.expected_revision,
            additions: request.additions.iter().map(protocol_node_draft).collect(),
            dependencies: request
                .dependencies
                .iter()
                .map(|(prerequisite, dependent)| {
                    (
                        protocol_node_reference(prerequisite),
                        protocol_node_reference(dependent),
                    )
                })
                .collect(),
            removed_dependencies: request
                .removed_dependencies
                .iter()
                .map(|(before, after)| (before.to_string(), after.to_string()))
                .collect(),
            superseded: request
                .superseded
                .iter()
                .map(|(node, replacement)| (node.to_string(), replacement.clone()))
                .collect(),
            assignment_changes: request
                .assignment_changes
                .iter()
                .map(|(node, assignment)| {
                    (
                        node.to_string(),
                        assignment.as_ref().map(protocol_assignment),
                    )
                })
                .collect(),
            preserved_terminal_nodes: request
                .preserved_terminal_nodes
                .iter()
                .map(ToString::to_string)
                .collect(),
            reason: request.reason.clone(),
        })
        .map_err(super::remote_error)?;
    service_remote_mapping::session_plan_from_response(response)
}

/// Converts a shared node proposal to its protocol representation.
///
/// - `node`: Structured plan node proposal.
///
/// The protocol node proposal.
/// # Returns
/// The operation result.
fn protocol_node_draft(node: &scriberig_app::PlanNodeDraft) -> SessionPlanNodeDraft {
    SessionPlanNodeDraft {
        key: node.key.clone(),
        parent_index: node.parent_index,
        title: node.title.clone(),
        purpose: node.purpose.clone(),
        expected_result: node.expected_result.as_ref().map(|result| {
            scriberig_protocol::SessionExpectedResult {
                kind: result.kind.clone(),
                requires_handle: result.requires_handle,
            }
        }),
        semantic_scope: node.semantic_scope.clone(),
        assignment: node.assignment.as_ref().map(protocol_assignment),
    }
}

/// Converts one shared dependency endpoint to its protocol representation.
///
/// - `reference`: Existing node identity or proposal-local key.
///
/// The equivalent protocol endpoint.
/// # Returns
/// The operation result.
fn protocol_node_reference(
    reference: &scriberig_app::PlanNodeReference,
) -> SessionPlanNodeReference {
    match reference {
        scriberig_app::PlanNodeReference::Existing(id) => {
            SessionPlanNodeReference::Existing { id: id.to_string() }
        }
        scriberig_app::PlanNodeReference::Proposed(key) => {
            SessionPlanNodeReference::Proposed { key: key.clone() }
        }
    }
}

/// Converts a credential-safe model assignment to its protocol value.
///
/// - `assignment`: Validated model-library selection.
///
/// Protocol assignment containing only stable configuration identifiers.
/// # Returns
/// The operation result.
fn protocol_assignment(assignment: &scriberig_session::ModelAssignment) -> SessionModelAssignment {
    SessionModelAssignment {
        provider_id: assignment.provider_id.clone(),
        model_id: assignment.model_id.clone(),
        thinking_level: assignment.thinking_level.clone(),
    }
}
