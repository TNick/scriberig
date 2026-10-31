//! Projects shared plan views and durable dependency knowledge into upstream Markdown.

use std::sync::Arc;

use crate::selection::PlanSelection;
use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanGateCounts, PlanNodeView, PlanView, Session};

use super::views::SharedPlanViews;

#[path = "details/attempts.rs"]
mod attempts;
#[path = "details/evidence.rs"]
mod evidence;
#[path = "details/gates.rs"]
mod gates;

/// Builds bounded details for a plan and its explicit dependency graph.
///
/// # Parameters
/// - `session`: Canonical session snapshot supplying the plans.
/// - `cursor`: Stable selected plan or node identity.
/// - `rows`: Stable projected plan/node rows carrying the shared views.
///
/// # Returns
/// Safe Markdown details for the selected plan or node.
pub(super) fn plan_details(
    session: &Session,
    cursor: Option<PlanSelection>,
    rows: &[super::PlanTreeRow],
) -> String {
    let Some(selection) = cursor
        .filter(|selection| rows.iter().any(|row| row.selection == *selection))
        .or_else(|| rows.first().map(|row| row.selection))
    else {
        return "# Plans\n\nNo durable plans are available.".to_owned();
    };
    let Some(row) = rows.iter().find(|row| row.selection == selection) else {
        return "# Plans\n\nSelected plan is no longer available.".to_owned();
    };
    let Some(plan) = session.plans.iter().find(|plan| plan.id == selection.plan) else {
        return "# Plans\n\nSelected plan is no longer available.".to_owned();
    };
    let mut output = String::from("# Plan\n\n");
    append_plan_view(&mut output, &row.semantics, plan);
    if let Some(node_id) = selection.node {
        append_node_details(
            &mut output,
            plan,
            node_id,
            row.semantics.node(node_id),
            row.semantics.plan.as_ref(),
        );
        gates::append_node_gates(&mut output, plan, node_id);
        evidence::append_node_evidence(&mut output, plan, node_id);
        attempts::append_node_attempts(&mut output, plan, node_id);
    }
    // History follows the selection: the bounded details pane must keep the
    // selected node's shared semantics visible instead of scrolling them off a
    // multi-revision plan history list.
    append_plan_history(&mut output, plan);
    append_dependency_edges(&mut output, plan);
    output
}

/// Appends the shared plan view plus the coordinator claim the view omits.
///
/// # Parameters
/// - `output`: Markdown rendered by upstream `rpi_tui::Markdown`.
/// - `semantics`: Shared views for the projected plan copy.
/// - `plan`: Canonical durable plan supplying the coordinator claim.
fn append_plan_view(output: &mut String, semantics: &Arc<SharedPlanViews>, plan: &Plan) {
    match semantics.plan.as_ref() {
        Some(view) => append_plan_view_fields(output, view),
        None => super::super::common::append_status(
            output,
            "Shared plan semantics",
            "unavailable for this plan",
            super::super::common::SemanticRole::Warning,
        ),
    }
    if let Some(claim) = &plan.coordinator_claim {
        super::super::common::append_value(
            output,
            &format!("Coordinator claim: {} · {}", claim.id, claim.owner),
        );
        if let Some(operation) = &claim.operation {
            super::super::common::append_value(
                output,
                &format!("Coordinator operation: {operation}"),
            );
        }
    }
}

/// Appends the identity, exact revision, status, goal, root, and status counts
/// reported by the shared `PlanView`.
///
/// # Parameters
/// - `output`: Markdown rendered by upstream `rpi_tui::Markdown`.
/// - `view`: Shared compact plan view for the selected plan copy.
fn append_plan_view_fields(output: &mut String, view: &PlanView) {
    super::super::common::append_value(output, &format!("Identity: {}", view.id));
    super::super::common::append_value(output, &format!("Exact revision: {}", view.revision));
    super::super::common::append_value(
        output,
        &format!("Mutation revision: {}", view.mutation_revision),
    );
    if let Some(snapshot) = view.snapshot {
        super::super::common::append_value(output, &format!("Snapshot: {snapshot}"));
    }
    super::status::append_plan_status(output, view.status);
    super::super::common::append_value(output, &format!("Goal: {}", view.goal));
    super::super::common::append_value(
        output,
        &format!(
            "Nodes: {} · planned {} · ready {} · running {} · completed {} · failed {} · abandoned {}",
            view.node_count,
            view.status_counts.planned,
            view.status_counts.ready,
            view.status_counts.running,
            view.status_counts.completed,
            view.status_counts.failed,
            view.status_counts.abandoned,
        ),
    );
    super::super::common::append_value(
        output,
        &format!("Root nodes: {}", joined_identities(&view.root_nodes)),
    );
    if let Some(cursor) = view.coordinator_cursor {
        super::super::common::append_value(output, &format!("Coordinator node: {cursor}"));
    }
    if let Some(coordinator) = view.coordinator {
        super::super::common::append_value(output, &format!("Coordinator branch: {coordinator}"));
    }
}

/// Projects bounded immutable replan history into selected plan details.
///
/// # Parameters
/// - `output`: Markdown rendered by upstream `rpi_tui::Markdown`.
/// - `plan`: Canonical durable `ScribeRig` plan with prior revisions.
fn append_plan_history(output: &mut String, plan: &Plan) {
    if plan.history.is_empty() {
        return;
    }
    super::super::common::append_value(output, "## Prior plan revisions");
    for revision in plan.history.iter().rev().take(20).rev() {
        super::super::common::append_value(
            output,
            &format!(
                "Revision {} · {:?} · {} nodes · {} dependencies",
                revision.revision,
                revision.status,
                revision.nodes.len(),
                revision.dependencies.len()
            ),
        );
        super::super::common::append_value(output, &format!("Goal: {}", revision.goal));
        if let Some(reason) = &revision.reason {
            super::super::common::append_value(output, &format!("Replan reason: {reason}"));
        }
    }
    if plan.history.len() > 20 {
        super::super::common::append_value(
            output,
            &format!("{} earlier revisions omitted.", plan.history.len() - 20),
        );
    }
}

/// Appends shared-view node semantics plus the canonical fields the view omits.
///
/// # Parameters
/// - `output`: Markdown receiving safe plan details.
/// - `plan`: Canonical durable plan containing the selected node.
/// - `node_id`: Selected durable `PlanNode` identity.
/// - `view`: Shared node view for the selected node, when assembly succeeded.
/// - `plan_view`: Shared plan view, which supplies the coordinator position.
fn append_node_details(
    output: &mut String,
    plan: &Plan,
    node_id: EntityId,
    view: Option<&PlanNodeView>,
    plan_view: Option<&PlanView>,
) {
    let Some(node) = plan.nodes.iter().find(|node| node.id == node_id) else {
        return;
    };
    let Some(view) = view else {
        super::super::common::append_value(output, &format!("## Node {node_id}"));
        super::super::common::append_status(
            output,
            "Shared plan semantics",
            "unavailable for this node",
            super::super::common::SemanticRole::Warning,
        );
        return;
    };
    super::super::common::append_value(output, &format!("## Node {}", view.id));
    super::super::common::append_value(output, &format!("Exact revision: {}", view.revision));
    if let Some(snapshot) = view.snapshot {
        super::super::common::append_value(output, &format!("Snapshot: {snapshot}"));
    }
    super::super::common::append_value(output, &format!("Title: {}", view.title));
    super::super::common::append_value(output, &format!("Purpose: {}", view.purpose));
    super::status::append_node_status(output, view.direct_status);
    super::status::append_effective_node_status(output, view.effective_status);
    if view.contains_ignored {
        super::super::common::append_status(
            output,
            "Contains ignored",
            "yes",
            super::super::common::SemanticRole::Warning,
        );
    }
    if view.contains_abandoned {
        super::super::common::append_status(
            output,
            "Contains abandoned",
            "yes",
            super::super::common::SemanticRole::Warning,
        );
    }
    match view.parent {
        Some(scriberig_session::PlanNodeParent::Branch(branch)) => {
            super::super::common::append_value(
                output,
                &format!("Structural parent branch: {branch}"),
            );
        }
        Some(scriberig_session::PlanNodeParent::Node(parent)) => {
            super::super::common::append_value(
                output,
                &format!("Structural parent node: {parent}"),
            );
        }
        None => super::super::common::append_value(output, "Structural parent: none"),
    }
    if let Some(branch) = view.branch {
        super::super::common::append_value(output, &format!("Execution branch: {branch}"));
    }
    if let Some(result) = view.result {
        super::super::common::append_value(output, &format!("BranchResult: {result}"));
    }
    if let Some(replacement) = node.superseded_by {
        super::super::common::append_value(output, &format!("Superseded by node: {replacement}"));
    }
    if let Some(assignment) = &view.assignment {
        super::super::common::append_value(
            output,
            &format!(
                "Assignment: {} / {} / {}",
                assignment.provider_id,
                assignment.model_id,
                assignment
                    .thinking_level
                    .as_deref()
                    .unwrap_or("unspecified")
            ),
        );
    }
    if let Some(claim) = &node.claim {
        super::super::common::append_value(
            output,
            &format!("Worker claim: {} · {}", claim.id, claim.owner),
        );
    }
    if let Some(expected) = &node.expected_result {
        super::super::common::append_value(output, &format!("Expected result: {}", expected.kind));
    }
    append_node_summaries(output, view, plan_view);
}

/// Appends the summary, blocker, gate, evidence, and retry summaries the shared
/// node view carries.
///
/// # Parameters
/// - `output`: Markdown receiving safe plan details.
/// - `view`: Shared node view for the selected node.
/// - `plan_view`: Shared plan view, which reports the coordinator position.
fn append_node_summaries(output: &mut String, view: &PlanNodeView, plan_view: Option<&PlanView>) {
    if let Some(summary) = &view.summary_preview {
        super::super::common::append_value(output, &format!("Summary: {summary}"));
    }
    if let Some(summary_id) = view.summary_id {
        super::super::common::append_value(output, &format!("Summary identity: {summary_id}"));
    }
    super::super::common::append_value(
        output,
        &format!(
            "Blocked by: {}",
            if view.dependency_blockers.is_empty() {
                "none".to_owned()
            } else {
                joined_identities(&view.dependency_blockers)
            }
        ),
    );
    super::super::common::append_value(
        output,
        &format!("Entry gates: {}", gate_summary(&view.entry_gates)),
    );
    super::super::common::append_value(
        output,
        &format!("Completion gates: {}", gate_summary(&view.completion_gates)),
    );
    super::super::common::append_value(output, &format!("Evidence: {}", view.evidence_count));
    super::super::common::append_value(
        output,
        &format!(
            "Retry: {}",
            if view.can_retry {
                "available"
            } else {
                "not available"
            }
        ),
    );
    if let Some(failure) = &view.last_failure {
        super::super::common::append_value(
            output,
            &format!(
                "Last failure: {} · {}",
                failure.reference.entity,
                failure
                    .reference
                    .snapshot
                    .map_or_else(|| "no snapshot".to_owned(), |snapshot| snapshot.to_string())
            ),
        );
    }
    if plan_view.is_some_and(|plan| plan.coordinator_cursor == Some(view.id)) {
        super::super::common::append_value(output, "Coordinator node: yes");
    }
}

/// Lists the explicit dependency edges as plan knowledge, never as tree structure.
///
/// # Parameters
/// - `output`: Markdown receiving the dependency edges.
/// - `plan`: Canonical durable plan owning the dependency edges.
fn append_dependency_edges(output: &mut String, plan: &Plan) {
    super::super::common::append_value(output, "## Explicit dependency edges");
    for edge in plan
        .dependencies
        .iter()
        .take(super::super::common::MAX_ROWS)
    {
        super::super::common::append_value(
            output,
            &format!(
                "{} → {} · result {}",
                edge.prerequisite,
                edge.dependent,
                edge.fulfilled_by
                    .map_or_else(|| "pending".to_owned(), |id| id.to_string())
            ),
        );
    }
}

/// Formats shared gate counts by evaluated state.
///
/// - `counts`: Entry or completion gate counts from the shared node view.
///
/// # Returns
/// One bounded gate summary line for the details pane.
fn gate_summary(counts: &PlanGateCounts) -> String {
    format!(
        "pending {} · satisfied {} · failed {} · waived {}",
        counts.pending, counts.satisfied, counts.failed, counts.waived
    )
}

/// Joins durable identities for one bounded details line.
///
/// - `identities`: Identities to render in their shared view order.
///
/// # Returns
/// A comma-separated identity list, or `none` when the list is empty.
fn joined_identities(identities: &[EntityId]) -> String {
    if identities.is_empty() {
        return "none".to_owned();
    }
    identities
        .iter()
        .take(super::super::common::MAX_ROWS)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
