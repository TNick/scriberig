//! Projects durable `ScribeRig` plans through upstream tree and Markdown APIs.

use rpi_tui::{TreeItem, TreeList};
use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanNode, PlanNodeParent, Session};

#[path = "plan/status.rs"]
mod status;
use crate::selection::PlanSelection;
use crate::state::TuiState;

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;

/// One stable plan or node row in the upstream tree.
pub(super) struct PlanTreeRow {
    /// Namespaced upstream row ID.
    pub(super) id: String,
    /// Safe bounded label projected from durable plan metadata.
    label: String,
    /// Structural tree parent, never a dependency edge.
    pub(super) parent_id: Option<String>,
    /// Durable `ScribeRig` selection represented by the row.
    pub(super) selection: PlanSelection,
}

/// Builds a plan tree and its selected semantic details.
///
/// - `state`: `ScribeRig` presentation state containing the exact session view.
///
/// Upstream `TreeList`, selected details, and the stable row sequence.
/// # Returns
/// Plan tree and details when a session snapshot is available.
pub(super) fn plan_tree(state: &TuiState) -> Option<(TreeList, String, Vec<PlanTreeRow>)> {
    let session = state.session_snapshot.as_ref()?;
    let rows = rows_for_plans(&session.plans);
    if rows.is_empty() {
        return None;
    }
    let items = rows
        .iter()
        .map(|row| TreeItem {
            id: row.id.clone(),
            label: row.label.clone(),
            parent_id: row.parent_id.clone(),
        })
        .collect::<Vec<_>>();
    let tree = TreeList::new(items, super::common::MAX_ROWS).ok()?;
    for row in &rows {
        if rows
            .iter()
            .any(|child| child.parent_id.as_deref() == Some(&row.id))
        {
            let _ = tree.set_expanded(&row.id, true);
        }
    }
    let selected = state
        .plan_cursor
        .and_then(|selection| {
            rows.iter()
                .find(|row| row.selection == selection)
                .map(|row| row.id.as_str())
        })
        .or_else(|| rows.get(state.cursor).map(|row| row.id.as_str()));
    if let Some(selected) = selected {
        let _ = tree.select(selected);
    }
    let details = plan_details(session, state.plan_cursor, &rows);
    Some((tree, details, rows))
}

/// Retains the stable selected plan or node after upstream tree movement.
///
/// - `state`: Presentation state receiving the current plan selection.
pub fn retain_cursor_identity(state: &mut TuiState) {
    let Some(session) = state.session_snapshot.as_ref() else {
        state.plan_cursor = None;
        return;
    };
    state.plan_cursor = rows_for_plans(&session.plans)
        .get(state.cursor)
        .map(|row| row.selection);
}

/// Reconciles the durable plan selection after a session or plan refresh.
///
/// - `state`: Presentation state receiving the refreshed row position.
pub fn reconcile_cursor(state: &mut TuiState) {
    let Some(session) = state.session_snapshot.as_ref() else {
        state.cursor = 0;
        state.plan_cursor = None;
        return;
    };
    let rows = rows_for_plans(&session.plans);
    if rows.is_empty() {
        state.cursor = 0;
        state.plan_cursor = None;
        return;
    }
    let index = state
        .plan_cursor
        .and_then(|selection| rows.iter().position(|row| row.selection == selection))
        .unwrap_or_else(|| state.cursor.min(rows.len() - 1));
    state.cursor = index;
    state.plan_cursor = rows.get(index).map(|row| row.selection);
}

/// Projects structural plan parents without using dependency edges as parents.
///
/// - `plans`: Durable plans from the canonical session snapshot.
///
/// Stable rows in plan and structural-node order.
/// # Returns
/// Plan headers and nodes in bounded structural order.
fn rows_for_plans(plans: &[Plan]) -> Vec<PlanTreeRow> {
    let mut rows = Vec::new();
    for plan in plans {
        if rows.len() >= super::common::MAX_ROWS {
            break;
        }
        let plan_id = plan_row_id(plan.id, None);
        rows.push(PlanTreeRow {
            id: plan_id.clone(),
            label: bounded_label(&format!(
                "Plan · {} · {:?} · r{}",
                plan.goal, plan.status, plan.revision
            )),
            parent_id: None,
            selection: PlanSelection {
                plan: plan.id,
                node: None,
            },
        });
        append_plan_nodes(plan, &plan_id, &mut rows);
    }
    rows
}

/// Adds plan nodes in parent-before-child order, with malformed parents bounded.
///
/// - `plan`: Durable plan supplying structural parent references.
/// - `plan_id`: Stable tree ID for the plan header.
/// - `rows`: Ordered rows receiving the plan's nodes.
fn append_plan_nodes(plan: &Plan, plan_id: &str, rows: &mut Vec<PlanTreeRow>) {
    let remaining = super::common::MAX_ROWS.saturating_sub(rows.len());
    let mut pending = plan.nodes.iter().take(remaining).collect::<Vec<_>>();
    let mut emitted = std::collections::BTreeSet::new();
    while !pending.is_empty() {
        let mut deferred = Vec::new();
        let mut progressed = false;
        for node in pending {
            let parent_id = node_parent_id(node, plan_id, &emitted);
            if parent_id.is_none() && matches!(node.parent, Some(PlanNodeParent::Node(_))) {
                deferred.push(node);
                continue;
            }
            append_node_row(plan, node, parent_id, rows, &mut emitted);
            progressed = true;
        }
        if !progressed {
            for node in deferred {
                append_node_row(plan, node, Some(plan_id.to_owned()), rows, &mut emitted);
            }
            break;
        }
        pending = deferred;
    }
}

/// Resolves a node's structural parent when that parent is already projected.
///
/// - `node`: Durable plan node being projected.
/// - `plan_id`: Stable plan header row ID.
/// - `emitted`: Node IDs already projected beneath the plan.
///
/// The structural tree parent; branch parents attach under the plan header.
/// # Returns
/// Parent row identity for this node.
fn node_parent_id(
    node: &PlanNode,
    plan_id: &str,
    emitted: &std::collections::BTreeSet<EntityId>,
) -> Option<String> {
    match node.parent {
        Some(PlanNodeParent::Node(parent)) if emitted.contains(&parent) => {
            Some(plan_row_id_for_node(plan_id, parent))
        }
        Some(PlanNodeParent::Node(_)) => None,
        Some(PlanNodeParent::Branch(_)) | None => Some(plan_id.to_owned()),
    }
}

/// Appends one node row after its structural parent is available.
///
/// - `plan`: Durable plan containing the node.
/// - `node`: Node being projected.
/// - `parent_id`: Structural tree parent row.
/// - `rows`: Ordered rows receiving the node.
/// - `emitted`: Node identities receiving the new row.
fn append_node_row(
    plan: &Plan,
    node: &PlanNode,
    parent_id: Option<String>,
    rows: &mut Vec<PlanTreeRow>,
    emitted: &mut std::collections::BTreeSet<EntityId>,
) {
    rows.push(PlanTreeRow {
        id: plan_row_id(plan.id, Some(node.id)),
        label: node_label(node),
        parent_id,
        selection: PlanSelection {
            plan: plan.id,
            node: Some(node.id),
        },
    });
    emitted.insert(node.id);
}

/// Builds a safe node label that exposes its structural parent category.
///
/// - `node`: Durable plan node represented by the row.
///
/// A bounded label including node identity and parent type.
/// # Returns
/// Human-readable bounded node label.
fn node_label(node: &PlanNode) -> String {
    let parent = match node.parent {
        Some(PlanNodeParent::Branch(branch)) => format!(" · branch parent {branch}"),
        Some(PlanNodeParent::Node(parent)) => format!(" · node parent {parent}"),
        None => " · root node".to_owned(),
    };
    bounded_label(&format!(
        "{:?} · {} · {}{}",
        node.status, node.title, node.id, parent
    ))
}

/// Creates a stable upstream key for a plan header or node.
///
/// - `plan`: Durable plan identity.
/// - `node`: Optional durable plan-node identity.
///
/// A namespaced selection key independent of row position.
/// # Returns
/// Stable namespaced row identity.
fn plan_row_id(plan: EntityId, node: Option<EntityId>) -> String {
    match node {
        Some(node) => format!("plan:{plan}:node:{node}"),
        None => format!("plan:{plan}"),
    }
}

/// Names a node row using its plan's already-namespaced tree ID.
///
/// - `plan_id`: Stable plan header row ID.
/// - `node`: Durable node identity.
///
/// A stable tree key for the node.
/// # Returns
/// Stable key under the plan header.
fn plan_row_id_for_node(plan_id: &str, node: EntityId) -> String {
    format!("{plan_id}:node:{node}")
}

/// Builds bounded details for a plan and its explicit dependency graph.
///
/// - `session`: Canonical session snapshot supplying the plans.
/// - `cursor`: Stable selected plan or node identity.
/// - `rows`: Stable projected plan/node rows.
///
/// Escaped Markdown for the upstream renderer.
/// # Returns
/// Safe details for the selected plan or node.
fn plan_details(session: &Session, cursor: Option<PlanSelection>, rows: &[PlanTreeRow]) -> String {
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
    super::common::append_value(&mut output, &format!("Identity: {}", plan.id));
    super::common::append_value(&mut output, &format!("Revision: {}", plan.revision));
    status::append_plan_status(&mut output, plan.status);
    super::common::append_value(&mut output, &format!("Goal: {}", plan.goal));
    if let Some(claim) = &plan.coordinator_claim {
        super::common::append_value(
            &mut output,
            &format!("Coordinator claim: {} · {}", claim.id, claim.owner),
        );
        if let Some(operation) = &claim.operation {
            super::common::append_value(
                &mut output,
                &format!("Coordinator operation: {operation}"),
            );
        }
    }
    if let Some(cursor) = plan.coordinator_cursor {
        super::common::append_value(&mut output, &format!("Coordinator node: {cursor}"));
    }
    if let Some(coordinator) = plan.coordinator {
        super::common::append_value(&mut output, &format!("Coordinator branch: {coordinator}"));
    }
    if let Some(node_id) = selection.node
        && let Some(node) = plan.nodes.iter().find(|node| node.id == node_id)
    {
        append_node_details(&mut output, node);
    }
    super::common::append_value(&mut output, "## Explicit dependency edges");
    for edge in plan.dependencies.iter().take(super::common::MAX_ROWS) {
        super::common::append_value(
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

/// Appends durable execution and structural details for one node.
///
/// - `output`: Markdown receiving safe plan details.
/// - `node`: Durable node from the selected plan.
fn append_node_details(output: &mut String, node: &PlanNode) {
    super::common::append_value(output, &format!("## Node {}", node.id));
    super::common::append_value(output, &format!("Title: {}", node.title));
    super::common::append_value(output, &format!("Purpose: {}", node.purpose));
    status::append_node_status(output, node.status);
    match node.parent {
        Some(PlanNodeParent::Branch(branch)) => {
            super::common::append_value(output, &format!("Structural parent branch: {branch}"));
        }
        Some(PlanNodeParent::Node(parent)) => {
            super::common::append_value(output, &format!("Structural parent node: {parent}"));
        }
        None => super::common::append_value(output, "Structural parent: none"),
    }
    if let Some(branch) = node.branch {
        super::common::append_value(output, &format!("Execution branch: {branch}"));
    }
    if let Some(result) = node.result {
        super::common::append_value(output, &format!("BranchResult: {result}"));
    }
    if let Some(assignment) = &node.assignment {
        super::common::append_value(
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
        super::common::append_value(
            output,
            &format!("Worker claim: {} · {}", claim.id, claim.owner),
        );
    }
    if let Some(expected) = &node.expected_result {
        super::common::append_value(output, &format!("Expected result: {}", expected.kind));
    }
}

/// Sanitizes and bounds one untrusted plan tree label.
///
/// - `value`: Plan or node text from the semantic session snapshot.
///
/// A terminal-safe label limited to 800 characters.
/// # Returns
/// Sanitized and bounded label.
fn bounded_label(value: &str) -> String {
    crate::inspector::sanitize(value)
        .chars()
        .take(800)
        .collect()
}
