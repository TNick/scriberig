//! Reads compact plan graph projections at exact semantic snapshots.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, ApplicationError};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{
    SessionPlanGateCounts, SessionPlanGraphView, SessionPlanNodeView, SessionPlanStatusCounts,
    SessionPlanView, SessionPlanViewRequest,
};

/// Reads a compact graph from the shared application plan projection.
///
/// # Parameters
/// - `service`: Shared application service owning session authorization.
/// - `context`: Trusted session read context.
/// - `request`: Session, plan, and optional exact snapshot.
///
/// # Returns
/// Compact plan and node views without history or full evidence collections.
///
/// # Errors
/// Returns authorization, lookup, identity, or projection errors.
pub(super) fn read<B: scriberig_app::ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    request: &SessionPlanViewRequest,
) -> Result<SessionPlanGraphView, ApplicationError> {
    let session = parse_identity(&request.session.id, "session")?;
    let plan = parse_identity(&request.plan, "plan")?;
    let snapshot = request
        .snapshot
        .as_deref()
        .map(|value| {
            value
                .parse::<SnapshotId>()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "plan view".to_owned(),
                    message: format!("invalid snapshot identifier: {error}"),
                })
        })
        .transpose()?;
    let (plan_view, node_views) = service.read_plan_views(context, session, plan, snapshot)?;
    Ok(SessionPlanGraphView {
        plan: SessionPlanView {
            id: plan_view.id.to_string(),
            revision: plan_view.revision.to_string(),
            snapshot: plan_view.snapshot.map(|value| value.to_string()),
            mutation_revision: plan_view.mutation_revision,
            goal: plan_view.goal,
            status: crate::server::server_helpers::session_plan::plan_status(plan_view.status),
            coordinator: plan_view.coordinator.map(|value| value.to_string()),
            coordinator_cursor: plan_view.coordinator_cursor.map(|value| value.to_string()),
            root_nodes: plan_view
                .root_nodes
                .iter()
                .map(ToString::to_string)
                .collect(),
            node_count: plan_view.node_count,
            status_counts: SessionPlanStatusCounts {
                planned: plan_view.status_counts.planned,
                ready: plan_view.status_counts.ready,
                running: plan_view.status_counts.running,
                completed: plan_view.status_counts.completed,
                failed: plan_view.status_counts.failed,
                abandoned: plan_view.status_counts.abandoned,
            },
        },
        nodes: node_views.into_iter().map(node_view).collect(),
    })
}

/// Converts a shared compact node view to its public protocol representation.
///
/// # Parameters
/// - `value`: Exact semantic node projection.
///
/// # Returns
/// Protocol-safe node summary without full gate or evidence records.
fn node_view(value: scriberig_session::PlanNodeView) -> SessionPlanNodeView {
    SessionPlanNodeView {
        id: value.id.to_string(),
        revision: value.revision.to_string(),
        snapshot: value.snapshot.map(|item| item.to_string()),
        plan: value.plan.to_string(),
        parent: value.parent.map(|parent| match parent {
            scriberig_session::PlanNodeParent::Branch(id) => format!("branch:{id}"),
            scriberig_session::PlanNodeParent::Node(id) => format!("node:{id}"),
        }),
        title: value.title,
        purpose: value.purpose,
        direct_status: crate::server::server_helpers::session_plan::node_status(
            value.direct_status,
        ),
        effective_status: crate::server::server_helpers::session_plan::node_status(
            value.effective_status,
        ),
        contains_ignored: value.contains_ignored,
        contains_abandoned: value.contains_abandoned,
        summary_id: value.summary_id.map(|item| item.to_string()),
        summary_preview: value.summary_preview,
        branch: value.branch.map(|item| item.to_string()),
        result: value.result.map(|item| item.to_string()),
        assignment: value
            .assignment
            .map(|item| scriberig_protocol::SessionModelAssignment {
                provider_id: item.provider_id,
                model_id: item.model_id,
                thinking_level: item.thinking_level,
            }),
        dependency_blockers: value
            .dependency_blockers
            .iter()
            .map(ToString::to_string)
            .collect(),
        entry_gates: gate_counts(value.entry_gates),
        completion_gates: gate_counts(value.completion_gates),
        evidence_count: value.evidence_count,
        can_retry: value.can_retry,
        last_failure: value
            .last_failure
            .as_ref()
            .map(crate::server::server_helpers::session_plan::semantics::evidence_reference),
    }
}

/// Converts shared evaluated gate counts to protocol values.
///
/// # Parameters
/// - `value`: Shared evaluated counts.
///
/// # Returns
/// Counts for pending, satisfied, failed, and waived gates.
fn gate_counts(value: scriberig_session::PlanGateCounts) -> SessionPlanGateCounts {
    SessionPlanGateCounts {
        pending: value.pending,
        satisfied: value.satisfied,
        failed: value.failed,
        waived: value.waived,
    }
}

/// Parses one stable entity identity or returns a structured request error.
///
/// # Parameters
/// - `value`: Serialized entity identity.
/// - `kind`: Identity kind named in any validation error.
///
/// # Returns
/// Parsed stable entity identity.
///
/// # Errors
/// Returns invalid-request when the input is malformed.
fn parse_identity(value: &str, kind: &str) -> Result<EntityId, ApplicationError> {
    EntityId::from_str(value).map_err(|error| ApplicationError::InvalidRequest {
        operation: "plan view".to_owned(),
        message: format!("invalid {kind} identifier: {error}"),
    })
}
