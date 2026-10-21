//! Projects durable plan state into safe protocol summaries.

use scriberig_protocol::{
    SessionContinuationAnchor, SessionCoordinatorClaim, SessionExpectedResult,
    SessionModelAssignment, SessionPlan, SessionPlanDependency, SessionPlanNode,
    SessionPlanNodeStatus, SessionPlanRevision, SessionPlanStatus, SessionWorkerClaim,
};
use scriberig_session::{Plan, PlanDependency, PlanNode, PlanRevision};

/// Converts one plan and its retained history into a safe wire projection.
///
/// - `plan`: Durable plan value from the shared session subsystem.
///
/// Protocol state including current and historical graph revisions.
/// # Returns
/// The operation result.
pub(crate) fn project(plan: Plan) -> SessionPlan {
    SessionPlan {
        id: plan.id.to_string(),
        revision: plan.revision,
        goal: plan.goal,
        status: plan_status(plan.status),
        anchor: plan.anchor.map(anchor),
        root_branch: plan.root_branch.to_string(),
        nodes: plan.nodes.into_iter().map(node).collect(),
        dependencies: plan.dependencies.iter().map(dependency).collect(),
        coordinator: plan.coordinator.map(|value| value.to_string()),
        coordinator_cursor: plan.coordinator_cursor.map(|value| value.to_string()),
        coordinator_claim: plan.coordinator_claim.map(|claim| SessionCoordinatorClaim {
            id: claim.id.to_string(),
            operation: claim.operation.map(|value| value.to_string()),
            owner: claim.owner,
            generation: claim.generation,
        }),
        history: plan.history.into_iter().map(revision).collect(),
    }
}

/// Converts one immutable plan revision.
///
/// - `value`: Historical session-domain revision.
///
/// Safe historical protocol state.
/// # Returns
/// The operation result.
fn revision(value: PlanRevision) -> SessionPlanRevision {
    SessionPlanRevision {
        revision: value.revision,
        goal: value.goal,
        status: plan_status(value.status),
        nodes: value.nodes.into_iter().map(node).collect(),
        dependencies: value.dependencies.iter().map(dependency).collect(),
        reason: value.reason,
    }
}

/// Converts one plan node while omitting provider secrets.
///
/// - `value`: Durable session-domain plan node.
///
/// Protocol node with stable structural and execution references.
/// # Returns
/// The operation result.
fn node(value: PlanNode) -> SessionPlanNode {
    SessionPlanNode {
        id: value.id.to_string(),
        parent: value
            .parent
            .map(|parent| match parent {
                scriberig_session::PlanNodeParent::Branch(branch) => {
                    format!("branch:{branch}")
                }
                scriberig_session::PlanNodeParent::Node(node) => format!("node:{node}"),
            })
            .unwrap_or_default(),
        title: value.title,
        purpose: value.purpose,
        status: node_status(value.status),
        branch: value.branch.map(|id| id.to_string()),
        result: value.result.map(|id| id.to_string()),
        superseded_by: value.superseded_by.map(|id| id.to_string()),
        expected_result: value.expected_result.map(|expected| SessionExpectedResult {
            kind: expected.kind,
            requires_handle: expected.requires_handle,
        }),
        semantic_scope: value.semantic_scope,
        assignment: value.assignment.map(assignment),
        claim: value.claim.map(|claim| SessionWorkerClaim {
            id: claim.id.to_string(),
            node: claim.node.to_string(),
            branch: claim.branch.to_string(),
            owner: claim.owner,
            generation: claim.generation,
            assignment: claim.assignment.map(assignment),
        }),
    }
}

/// Converts a safe model assignment.
///
/// - `value`: Upstream provider, model, and thinking identities.
///
/// Credential-free protocol assignment.
/// # Returns
/// The operation result.
fn assignment(value: scriberig_session::ModelAssignment) -> SessionModelAssignment {
    SessionModelAssignment {
        provider_id: value.provider_id,
        model_id: value.model_id,
        thinking_level: value.thinking_level,
    }
}

/// Converts one explicit dependency edge.
///
/// - `value`: Durable result-based dependency.
///
/// Protocol edge with its exact fulfillment result.
/// # Returns
/// The operation result.
fn dependency(value: &PlanDependency) -> SessionPlanDependency {
    SessionPlanDependency {
        prerequisite: value.prerequisite.to_string(),
        dependent: value.dependent.to_string(),
        fulfilled_by: value.fulfilled_by.as_ref().map(ToString::to_string),
    }
}

/// Converts a shared continuation location.
///
/// - `value`: Exact session-domain continuation anchor.
///
/// Stable protocol continuation identity.
/// # Returns
/// The operation result.
fn anchor(value: scriberig_session::ContinuationAnchor) -> SessionContinuationAnchor {
    match value {
        scriberig_session::ContinuationAnchor::BranchHead { branch } => {
            SessionContinuationAnchor::BranchHead {
                branch: branch.to_string(),
            }
        }
        scriberig_session::ContinuationAnchor::AssistantOutput { branch, turn } => {
            SessionContinuationAnchor::AssistantOutput {
                branch: branch.to_string(),
                turn: turn.to_string(),
            }
        }
        scriberig_session::ContinuationAnchor::HumanInput { branch, turn } => {
            SessionContinuationAnchor::HumanInput {
                branch: branch.to_string(),
                turn: turn.to_string(),
            }
        }
        scriberig_session::ContinuationAnchor::BranchResult { branch, result } => {
            SessionContinuationAnchor::BranchResult {
                branch: branch.to_string(),
                result: result.to_string(),
            }
        }
    }
}

/// Converts plan lifecycle state.
///
/// - `value`: Shared session-domain plan state.
///
/// Provider-neutral protocol state.
/// # Returns
/// The operation result.
fn plan_status(value: scriberig_session::PlanStatus) -> SessionPlanStatus {
    match value {
        scriberig_session::PlanStatus::Draft => SessionPlanStatus::Draft,
        scriberig_session::PlanStatus::Planned => SessionPlanStatus::Planned,
        scriberig_session::PlanStatus::Running => SessionPlanStatus::Running,
        scriberig_session::PlanStatus::Paused => SessionPlanStatus::Paused,
        scriberig_session::PlanStatus::Completed => SessionPlanStatus::Completed,
        scriberig_session::PlanStatus::Failed => SessionPlanStatus::Failed,
        scriberig_session::PlanStatus::Cancelled => SessionPlanStatus::Cancelled,
    }
}

/// Converts node scheduling state.
///
/// - `value`: Shared session-domain node state.
///
/// Provider-neutral protocol state.
/// # Returns
/// The operation result.
fn node_status(value: scriberig_session::PlanNodeStatus) -> SessionPlanNodeStatus {
    match value {
        scriberig_session::PlanNodeStatus::Planned => SessionPlanNodeStatus::Planned,
        scriberig_session::PlanNodeStatus::Ready => SessionPlanNodeStatus::Ready,
        scriberig_session::PlanNodeStatus::Running => SessionPlanNodeStatus::Running,
        scriberig_session::PlanNodeStatus::Completed => SessionPlanNodeStatus::Completed,
        scriberig_session::PlanNodeStatus::Failed => SessionPlanNodeStatus::Failed,
        scriberig_session::PlanNodeStatus::Abandoned => SessionPlanNodeStatus::Abandoned,
    }
}
