//! Projects durable plan and dependency details into upstream Markdown.

use crate::selection::PlanSelection;
use scriberig_session::{Plan, PlanNode, PlanNodeParent, Session};

/// Builds bounded details for a plan and its explicit dependency graph.
///
/// # Parameters
/// - `session`: Canonical session snapshot supplying the plans.
/// - `cursor`: Stable selected plan or node identity.
/// - `rows`: Stable projected plan/node rows.
///
/// # Returns
/// Safe Markdown details for the selected plan or node.
pub(super) fn plan_details(
    session: &Session,
    cursor: Option<PlanSelection>,
    rows: &[super::PlanTreeRow],
) -> String {
    let selection = cursor
        .filter(|selection| rows.iter().any(|row| row.selection == *selection))
        .or_else(|| rows.first().map(|row| row.selection));
    let Some(selection) = selection else {
        return "# Plans\n\nNo durable plans are available.".to_owned();
    };
    let Some(plan) = session.plans.iter().find(|plan| plan.id == selection.plan) else {
        return "# Plans\n\nSelected plan is no longer available.".to_owned();
    };
    let mut output = String::from("# Plan\n\n");
    super::super::common::append_value(&mut output, &format!("Identity: {}", plan.id));
    super::super::common::append_value(&mut output, &format!("Revision: {}", plan.revision));
    super::status::append_plan_status(&mut output, plan.status);
    super::super::common::append_value(&mut output, &format!("Goal: {}", plan.goal));
    if let Some(claim) = &plan.coordinator_claim {
        super::super::common::append_value(
            &mut output,
            &format!("Coordinator claim: {} · {}", claim.id, claim.owner),
        );
        if let Some(operation) = &claim.operation {
            super::super::common::append_value(
                &mut output,
                &format!("Coordinator operation: {operation}"),
            );
        }
    }
    if let Some(cursor) = plan.coordinator_cursor {
        super::super::common::append_value(&mut output, &format!("Coordinator node: {cursor}"));
    }
    if let Some(coordinator) = plan.coordinator {
        super::super::common::append_value(
            &mut output,
            &format!("Coordinator branch: {coordinator}"),
        );
    }
    append_plan_history(&mut output, plan);
    if let Some(node_id) = selection.node
        && let Some(node) = plan.nodes.iter().find(|node| node.id == node_id)
    {
        append_node_details(&mut output, node);
    }
    super::super::common::append_value(&mut output, "## Explicit dependency edges");
    for edge in plan
        .dependencies
        .iter()
        .take(super::super::common::MAX_ROWS)
    {
        super::super::common::append_value(
            &mut output,
            &format!(
                "{} → {} · result {}",
                edge.prerequisite,
                edge.dependent,
                edge.fulfilled_by
                    .map_or_else(|| "pending".to_owned(), |id| id.to_string())
            ),
        );
    }
    output
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

/// Appends durable execution and structural details for one plan node.
///
/// # Parameters
/// - `output`: Markdown receiving safe plan details.
/// - `node`: Durable node from the selected plan.
fn append_node_details(output: &mut String, node: &PlanNode) {
    super::super::common::append_value(output, &format!("## Node {}", node.id));
    super::super::common::append_value(output, &format!("Title: {}", node.title));
    super::super::common::append_value(output, &format!("Purpose: {}", node.purpose));
    super::status::append_node_status(output, node.status);
    match node.parent {
        Some(PlanNodeParent::Branch(branch)) => {
            super::super::common::append_value(
                output,
                &format!("Structural parent branch: {branch}"),
            );
        }
        Some(PlanNodeParent::Node(parent)) => {
            super::super::common::append_value(
                output,
                &format!("Structural parent node: {parent}"),
            );
        }
        None => super::super::common::append_value(output, "Structural parent: none"),
    }
    if let Some(branch) = node.branch {
        super::super::common::append_value(output, &format!("Execution branch: {branch}"));
    }
    if let Some(result) = node.result {
        super::super::common::append_value(output, &format!("BranchResult: {result}"));
    }
    if let Some(replacement) = node.superseded_by {
        super::super::common::append_value(output, &format!("Superseded by node: {replacement}"));
    }
    if let Some(assignment) = &node.assignment {
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
}
