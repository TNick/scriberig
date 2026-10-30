//! Builds compact plan metadata from exact semantic objects.

use super::super::super::semantic::PlanSemanticSnapshot;
use super::super::super::semantic::encode::PLAN_TYPE;
use super::super::super::{PlanNodeParent, PlanNodeStatus, PlanNodeStatusCounts, PlanView};
use super::{PlanViewPayload, decode_payload, invalid, validate_revision};

/// Decodes plan metadata and node statuses without loading detailed records.
///
/// # Parameters
/// - `snapshot`: Exact semantic objects selected for this plan view.
///
/// # Returns
/// A compact view with immutable revision identities and direct node counts.
///
/// # Errors
/// Returns an error when plan metadata, membership, or structure is invalid.
pub(in crate::planning::semantic) fn plan_view(
    snapshot: &PlanSemanticSnapshot,
) -> Result<PlanView, super::super::PlanSemanticError> {
    let mut plan_object = None;
    for object in &snapshot.objects {
        if object.object_type.as_str() != PLAN_TYPE {
            continue;
        }
        validate_revision(object)?;
        if object.entity != snapshot.plan || plan_object.is_some() {
            return Err(invalid("missing, duplicate, or mismatched plan object"));
        }
        plan_object = Some(object);
    }
    let object = plan_object.ok_or_else(|| invalid("plan object is missing"))?;
    let payload: PlanViewPayload = decode_payload(object)?;
    if payload.schema_version != 1 {
        return Err(invalid("unsupported plan payload version"));
    }
    let nodes = super::node_refs::node_refs(snapshot)?;
    let mut status_counts = PlanNodeStatusCounts::default();
    for node in &nodes {
        increment_count(&mut status_counts, node.status);
    }
    let root_nodes = nodes
        .iter()
        .filter(|node| matches!(node.parent, Some(PlanNodeParent::Branch(_)) | None))
        .map(|node| node.entity)
        .collect();
    Ok(PlanView {
        id: snapshot.plan,
        revision: object.revision,
        snapshot: snapshot.snapshot,
        mutation_revision: payload.mutation_revision,
        goal: payload.goal,
        status: payload.status,
        coordinator: payload.coordinator,
        coordinator_cursor: payload.coordinator_cursor,
        root_nodes,
        node_count: u32::try_from(nodes.len()).unwrap_or(u32::MAX),
        status_counts,
    })
}

/// Counts one direct node state without overflowing the compact view.
///
/// # Parameters
/// - `counts`: Direct status accumulator.
/// - `status`: Canonical node lifecycle state.
fn increment_count(counts: &mut PlanNodeStatusCounts, status: PlanNodeStatus) {
    let count = match status {
        PlanNodeStatus::Planned => &mut counts.planned,
        PlanNodeStatus::Ready => &mut counts.ready,
        PlanNodeStatus::Running => &mut counts.running,
        PlanNodeStatus::Completed => &mut counts.completed,
        PlanNodeStatus::Failed => &mut counts.failed,
        PlanNodeStatus::Abandoned => &mut counts.abandoned,
    };
    *count = count.saturating_add(1);
}
