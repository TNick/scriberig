//! Projects durable plan state into safe protocol summaries.

use scriberig_protocol::{
    SessionContinuationAnchor, SessionCoordinatorClaim, SessionExpectedResult,
    SessionModelAssignment, SessionPlan, SessionPlanAttempt, SessionPlanDependency,
    SessionPlanNode, SessionPlanNodeStatus, SessionPlanRevision, SessionPlanStatus,
    SessionWorkerClaim,
};
use scriberig_session::{Plan, PlanDependency, PlanNode, PlanNodeView, PlanRevision};

#[path = "server_helpers_session_plan_semantics.rs"]
mod semantics;

/// Converts one retained attempt into a bounded, transcript-free DTO.
///
/// # Parameters
/// - `value`: Shared attempt reconstructed from immutable plan history.
///
/// # Returns
/// The safe protocol attempt record.
pub(crate) fn attempt(value: &scriberig_session::PlanAttemptView) -> SessionPlanAttempt {
    SessionPlanAttempt {
        branch: value.branch.to_string(),
        claim_generation: value.claim_generation,
        claim: value.claim.map(|claim| claim.to_string()),
        status: node_status(value.status),
        result: value.result.map(|result| result.to_string()),
        failure: value.failure.as_ref().map(semantics::evidence_reference),
    }
}

/// Converts one plan and its retained history into a safe wire projection.
///
/// - `plan`: Durable plan value from the shared session subsystem.
///
/// Protocol state including current and historical graph revisions.
/// # Returns
/// The operation result.
/// - `results`: Exact branch results used for projection and readiness.
///
/// # Errors
/// Returns an error when the requested operation cannot complete.
pub(crate) fn project(
    plan: Plan,
    results: Option<&[scriberig_session::BranchResult]>,
) -> Result<SessionPlan, scriberig_app::ApplicationError> {
    let snapshot = plan.source_snapshot;
    let plan_view = plan.plan_view(snapshot).map_err(view_error)?;
    let rollups = plan.node_rollups().map_err(view_error)?;
    let node_views = plan
        .nodes
        .iter()
        .map(|plan_node| {
            plan.node_view(plan_node.id, snapshot, results, &rollups)
                .map(|view| (plan_node.id, view))
                .map_err(view_error)
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
    let nodes = plan
        .nodes
        .into_iter()
        .map(|plan_node| {
            let view = node_views
                .get(&plan_node.id)
                .ok_or_else(|| view_error("shared node view is missing"))?;
            Ok(node(plan_node, Some(view)))
        })
        .collect::<Result<Vec<_>, scriberig_app::ApplicationError>>()?;
    Ok(SessionPlan {
        id: plan.id.to_string(),
        revision: plan.revision,
        exact_revision: Some(plan_view.revision.to_string()),
        snapshot: plan_view.snapshot.map(|value| value.to_string()),
        goal: plan.goal,
        status: plan_status(plan.status),
        anchor: plan.anchor.map(anchor),
        root_branch: plan.root_branch.to_string(),
        nodes,
        dependencies: plan.dependencies.iter().map(dependency).collect(),
        root_nodes: plan.root_nodes.iter().map(ToString::to_string).collect(),
        status_counts: semantics::status_counts(plan_view.status_counts),
        gates: plan.gates.iter().map(semantics::gate).collect(),
        evidence: plan.evidence.iter().map(semantics::evidence).collect(),
        summaries: plan.summaries.iter().map(semantics::summary).collect(),
        coordinator: plan.coordinator.map(|value| value.to_string()),
        coordinator_cursor: plan.coordinator_cursor.map(|value| value.to_string()),
        coordinator_claim: plan.coordinator_claim.map(|claim| SessionCoordinatorClaim {
            id: claim.id.to_string(),
            operation: claim.operation.map(|value| value.to_string()),
            owner: claim.owner,
            generation: claim.generation,
        }),
        history: plan.history.into_iter().map(revision).collect(),
    })
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
        nodes: value
            .nodes
            .into_iter()
            .map(|plan_node| node(plan_node, None))
            .collect(),
        dependencies: value.dependencies.iter().map(dependency).collect(),
        summaries: value.summaries.iter().map(semantics::summary).collect(),
        gates: value.gates.iter().map(semantics::gate).collect(),
        evidence: value.evidence.iter().map(semantics::evidence).collect(),
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
/// - `view`: Exact node view used for derived presentation fields.
fn node(value: PlanNode, view: Option<&PlanNodeView>) -> SessionPlanNode {
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
        exact_revision: view.map(|view| view.revision.to_string()),
        effective_status: view.map(|view| node_status(view.effective_status)),
        contains_ignored: view.is_some_and(|view| view.contains_ignored),
        contains_abandoned: view.is_some_and(|view| view.contains_abandoned),
        abandon_reason: value.abandon_reason.map(semantics::abandon_reason),
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
        summary_id: view.and_then(|view| view.summary_id.map(|value| value.to_string())),
        summary_preview: view.and_then(|view| view.summary_preview.clone()),
        dependency_blockers: view.map_or_else(Vec::new, |view| {
            view.dependency_blockers
                .iter()
                .map(ToString::to_string)
                .collect()
        }),
        entry_gates: view.map_or_else(Default::default, |view| {
            semantics::gate_counts(view.entry_gates)
        }),
        completion_gates: view.map_or_else(Default::default, |view| {
            semantics::gate_counts(view.completion_gates)
        }),
        evidence_count: view.map_or(0, |view| view.evidence_count),
        can_retry: view.is_some_and(|view| view.can_retry),
        last_failure: value
            .last_failure
            .as_ref()
            .map(semantics::evidence_reference),
    }
}

/// Converts a shared view assembly error to a safe server application error.
///
/// # Parameters
/// - `error`: Typed plan projection failure.
///
/// A bounded internal protocol-projection error.
///
/// # Returns
/// The value described by the return type.
fn view_error(error: impl std::fmt::Debug) -> scriberig_app::ApplicationError {
    scriberig_app::ApplicationError::Internal {
        message: format!("plan view assembly failed: {error:?}"),
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
