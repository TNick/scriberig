//! Projects durable `ScribeRig` plans through upstream tree and Markdown APIs.

use rpi_tui::TreeItem;
#[cfg(test)]
use rpi_tui::TreeList;
use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanNode, PlanNodeParent};

#[path = "plan/details.rs"]
mod details;
#[path = "plan/status.rs"]
mod status;
use super::projection::TreeProjection;
#[cfg(test)]
use super::projection::tree_from_items;
use crate::selection::PlanSelection;
use crate::state::TuiState;
use details::plan_details;

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

/// Builds reusable plan tree items, selection, details, and stable rows.
///
/// - `state`: `ScribeRig` presentation state containing the exact session view.
///
/// A tree projection plus the stable row sequence used by details.
/// # Returns
/// Plan projection when a session snapshot with plans is available.
pub(super) fn projection(state: &TuiState) -> Option<(TreeProjection, Vec<PlanTreeRow>)> {
    let session = state.session_snapshot.as_ref()?;
    let rows = rows_for_plans(&session.plans);
    if rows.is_empty() {
        return None;
    }
    let selection = selected_plan_selection(state, &rows);
    let selected_id = selection.and_then(|selection| {
        rows.iter()
            .find(|row| row.selection == selection)
            .map(|row| row.id.clone())
    });
    let items = rows
        .iter()
        .map(|row| TreeItem {
            id: row.id.clone(),
            label: row.label.clone(),
            parent_id: row.parent_id.clone(),
        })
        .collect();
    let details = plan_details(session, selection, &rows);
    Some((
        TreeProjection {
            items,
            selected_id,
            details,
        },
        rows,
    ))
}

/// Builds a standalone plan tree for focused projection tests.
///
/// - `state`: `ScribeRig` presentation state containing the exact session view.
///
/// An upstream `TreeList`, selected details, and the stable row sequence.
/// # Returns
/// Plan tree and details when a session snapshot is available.
#[cfg(test)]
pub(super) fn plan_tree(state: &TuiState) -> Option<(TreeList, String, Vec<PlanTreeRow>)> {
    let (projection, rows) = projection(state)?;
    let tree = tree_from_items(&projection.items)?;
    if let Some(selected_id) = projection.selected_id {
        let _ = tree.select(&selected_id);
    }
    Some((tree, projection.details, rows))
}

/// Resolves the selected plan identity once for tree and detail projections.
///
/// - `state`: `ScribeRig` navigation state with a stable plan cursor and fallback
///   row position.
/// - `rows`: Current stable plan and node rows.
///
/// The matching stable plan or node identity, or `None` when there are no rows.
/// # Returns
/// Selection shared by the upstream tree highlight and Markdown details.
fn selected_plan_selection(state: &TuiState, rows: &[PlanTreeRow]) -> Option<PlanSelection> {
    state
        .plan_cursor
        .filter(|selection| rows.iter().any(|row| row.selection == *selection))
        .or_else(|| rows.get(state.cursor).map(|row| row.selection))
        .or_else(|| rows.first().map(|row| row.selection))
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
    super::common::unique_row_ids(rows.iter_mut().map(|row| &mut row.id));
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
        label: node_label(node, plan.coordinator_cursor),
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
/// - `coordinator_cursor`: Plan node currently owned by the coordinator.
///
/// A bounded label including node identity and parent type.
/// # Returns
/// Human-readable bounded node label.
fn node_label(node: &PlanNode, coordinator_cursor: Option<EntityId>) -> String {
    let parent = match node.parent {
        Some(PlanNodeParent::Branch(branch)) => format!(" · branch parent {branch}"),
        Some(PlanNodeParent::Node(parent)) => format!(" · node parent {parent}"),
        None => " · root node".to_owned(),
    };
    let coordinator = if coordinator_cursor == Some(node.id) {
        " · coordinator cursor"
    } else {
        ""
    };
    bounded_label(&format!(
        "{:?} · {} · {}{}{}",
        node.status, node.title, node.id, parent, coordinator
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
