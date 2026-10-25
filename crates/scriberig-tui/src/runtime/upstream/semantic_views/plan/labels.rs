//! Builds bounded upstream tree labels from the shared plan and node views.

use scriberig_core::EntityId;
use scriberig_session::{PlanGateCounts, PlanNode};

use super::views::SharedPlanViews;

/// Builds the plan-header label from the shared plan view.
///
/// - `semantics`: Shared views for the projected plan copy.
/// - `plan`: Durable plan identity, named when the shared assembly rejected the copy.
///
/// The durable goal, lifecycle status, and plan mutation revision exactly as the
/// shared `PlanView` reports them. A rejected copy names only its stable
/// identity, because the projection must not substitute local status values.
/// # Returns
/// Human-readable bounded plan header label.
pub(super) fn plan_label(semantics: &SharedPlanViews, plan: EntityId) -> String {
    let Some(view) = semantics.plan.as_ref() else {
        return super::bounded_label(&format!(
            "Plan · {plan} · shared plan semantics unavailable"
        ));
    };
    super::bounded_label(&format!(
        "Plan · {} · {:?} · r{}",
        view.goal, view.status, view.mutation_revision
    ))
}

/// Builds a node label from the shared node view.
///
/// - `semantics`: Shared views for the projected plan copy.
/// - `node`: Durable plan node represented by the row.
///
/// The canonical direct status, the structural parent category, the coordinator
/// position, then the derived values the shared `PlanNodeView` carries: effective
/// status when it differs from the direct status, subtree ignored/abandoned
/// markers, and gate and evidence counts. Nothing here is recomputed from the
/// projected rows.
/// # Returns
/// Human-readable bounded node label.
pub(super) fn node_label(semantics: &SharedPlanViews, node: &PlanNode) -> String {
    let Some(view) = semantics.node(node.id) else {
        return super::bounded_label(&format!("{} · shared plan semantics unavailable", node.id));
    };
    let parent = match view.parent {
        Some(scriberig_session::PlanNodeParent::Branch(branch)) => {
            format!(" · branch parent {branch}")
        }
        Some(scriberig_session::PlanNodeParent::Node(parent)) => {
            format!(" · node parent {parent}")
        }
        None => " · root node".to_owned(),
    };
    let coordinator = if semantics
        .plan
        .as_ref()
        .is_some_and(|plan| plan.coordinator_cursor == Some(view.id))
    {
        " · coordinator cursor"
    } else {
        ""
    };
    let mut markers: Vec<String> = Vec::new();
    if view.effective_status != view.direct_status {
        markers.push(format!(" · effective {:?}", view.effective_status));
    }
    if view.contains_ignored {
        markers.push(" · contains ignored".to_owned());
    }
    if view.contains_abandoned {
        markers.push(" · contains abandoned".to_owned());
    }
    let gates = gate_total(&view.entry_gates).saturating_add(gate_total(&view.completion_gates));
    if gates > 0 {
        markers.push(format!(
            " · {gates} gate{}",
            if gates == 1 { "" } else { "s" }
        ));
    }
    if view.evidence_count > 0 {
        markers.push(format!(" · {} evidence", view.evidence_count));
    }
    super::bounded_label(&format!(
        "{:?} · {} · {}{}{}{}",
        view.direct_status,
        view.title,
        view.id,
        parent,
        coordinator,
        markers.concat()
    ))
}

/// Sums one shared gate-count summary across every evaluated state.
///
/// - `counts`: Entry or completion gate counts from the shared node view.
///
/// # Returns
/// The total gate count the shared view reports for that phase.
fn gate_total(counts: &PlanGateCounts) -> u32 {
    counts
        .pending
        .saturating_add(counts.satisfied)
        .saturating_add(counts.failed)
        .saturating_add(counts.waived)
}
