//! Projects durable `ScribeRig` plans through upstream tree and Markdown APIs.

use std::sync::Arc;

use rpi_tui::TreeItem;
#[cfg(test)]
use rpi_tui::TreeList;
use scriberig_core::EntityId;
use scriberig_session::{BranchResult, Plan, PlanNode, PlanNodeParent};

#[path = "plan/details.rs"]
mod details;
#[path = "plan/labels.rs"]
mod labels;
#[path = "plan/status.rs"]
mod status;
#[path = "plan/views.rs"]
mod views;
use super::projection::TreeProjection;
#[cfg(test)]
use super::projection::tree_from_items;
use crate::selection::PlanSelection;
use crate::state::TuiState;
use details::plan_details;
use labels::{node_label, plan_label};
use views::SharedPlanViews;

#[cfg(test)]
#[path = "plan_relation_tests.rs"]
mod relation_tests;
#[cfg(test)]
#[path = "plan_requirements_tests.rs"]
mod requirements_tests;
#[cfg(test)]
#[path = "plan_semantics_tests.rs"]
mod semantics_tests;
#[cfg(test)]
#[path = "plan_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;

/// One stable plan or node row in the upstream tree.
///
/// # Public members
/// - `id`: Namespaced upstream row ID.
/// - `parent_id`: Structural tree parent, never a dependency edge.
/// - `selection`: Durable `ScribeRig` selection represented by the row.
///
/// # Private members
/// - `label`: Safe bounded label projected from the shared plan views.
/// - `semantics`: Shared plan views for the plan copy this row belongs to.
pub(super) struct PlanTreeRow {
    /// Namespaced upstream row ID.
    pub(super) id: String,
    /// Safe bounded label projected from the shared plan views.
    label: String,
    /// Structural tree parent, never a dependency edge.
    pub(super) parent_id: Option<String>,
    /// Durable `ScribeRig` selection represented by the row.
    pub(super) selection: PlanSelection,
    /// Shared plan views for the plan copy this row belongs to.
    semantics: Arc<SharedPlanViews>,
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
    let rows = rows_for_session(session);
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
    state.plan_cursor = rows_for_session(session)
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
    let rows = rows_for_session(session);
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

/// Projects every durable plan copy in one canonical session snapshot.
///
/// - `session`: Session snapshot supplying the plans and their visible branch results.
///
/// Stable rows in plan and structural-node order.
/// # Returns
/// Plan headers and nodes carrying the shared views of their plan copy.
fn rows_for_session(session: &scriberig_session::Session) -> Vec<PlanTreeRow> {
    let results = session
        .branches
        .iter()
        .filter_map(|branch| branch.result.clone())
        .collect::<Vec<_>>();
    rows_for_plans(&session.plans, &results)
}

/// Projects structural plan parents without using dependency edges as parents.
///
/// - `plans`: Durable plans from the canonical session snapshot.
/// - `results`: Exact branch results visible in the same snapshot.
///
/// Stable rows in plan and structural-node order.
/// # Returns
/// Plan headers and nodes carrying the shared views of their plan copy.
fn rows_for_plans(plans: &[Plan], results: &[BranchResult]) -> Vec<PlanTreeRow> {
    let mut rows = Vec::new();
    // Upstream tree validation rejects a duplicate item ID and discards the whole
    // item set. Keys are reserved while the copy is projected so a repeated plan
    // identity re-parents its own nodes instead of pointing at the first copy.
    let mut used = std::collections::HashSet::new();
    for plan in plans {
        if rows.len() >= super::common::MAX_ROWS {
            break;
        }
        let semantics = Arc::new(SharedPlanViews::assemble(plan, results));
        let plan_key = super::common::unique_row_key(&mut used, plan_row_id(plan.id, None));
        rows.push(PlanTreeRow {
            id: plan_key.clone(),
            label: plan_label(&semantics, plan.id),
            parent_id: None,
            selection: PlanSelection {
                plan: plan.id,
                node: None,
            },
            semantics: Arc::clone(&semantics),
        });
        let mut sink = PlanRowSink {
            rows: &mut rows,
            used: &mut used,
            emitted: std::collections::BTreeMap::new(),
        };
        append_plan_nodes(plan, &plan_key, &semantics, &mut sink);
    }
    rows
}

/// Adds plan nodes in parent-before-child order, with malformed parents bounded.
///
/// - `plan`: Durable plan supplying the projected node identities.
/// - `plan_key`: Reserved tree key for the plan header.
/// - `semantics`: Shared views for this plan copy.
/// - `sink`: Ordered rows, reserved keys, and projected-node keys for this copy.
fn append_plan_nodes(
    plan: &Plan,
    plan_key: &str,
    semantics: &Arc<SharedPlanViews>,
    sink: &mut PlanRowSink<'_>,
) {
    let remaining = super::common::MAX_ROWS.saturating_sub(sink.rows.len());
    let mut pending = plan.nodes.iter().take(remaining).collect::<Vec<_>>();
    while !pending.is_empty() {
        let mut deferred = Vec::new();
        let mut progressed = false;
        for node in pending {
            let structural_parent = structural_parent(semantics, node);
            let parent_id = node_parent_id(structural_parent, plan_key, &sink.emitted);
            if parent_id.is_none() && matches!(structural_parent, Some(PlanNodeParent::Node(_))) {
                deferred.push(node);
                continue;
            }
            sink.node_row(plan, node, plan_key, parent_id, semantics);
            progressed = true;
        }
        if !progressed {
            for node in deferred {
                sink.node_row(plan, node, plan_key, Some(plan_key.to_owned()), semantics);
            }
            break;
        }
        pending = deferred;
    }
}

/// Reads one node's structural parent from its shared node view.
///
/// - `semantics`: Shared views for the projected plan copy.
/// - `node`: Durable plan node being projected.
///
/// The parent relation named by `PlanNodeView::parent`. A plan copy the shared
/// assembly rejected has no validated structural relation, so its rows stay
/// under the plan header instead of the projection inventing parentage.
/// # Returns
/// Structural parent relation for this node.
fn structural_parent(semantics: &SharedPlanViews, node: &PlanNode) -> Option<PlanNodeParent> {
    semantics.node(node.id).and_then(|view| view.parent)
}

/// Resolves a node's structural parent when that parent is already projected.
///
/// - `parent`: Structural parent relation from the shared node view.
/// - `plan_key`: Reserved plan header row ID.
/// - `emitted`: Reserved row keys for nodes already projected beneath the plan.
///
/// The structural tree parent; branch parents and roots attach under the plan header.
/// # Returns
/// Parent row identity for this node.
fn node_parent_id(
    parent: Option<PlanNodeParent>,
    plan_key: &str,
    emitted: &std::collections::BTreeMap<EntityId, String>,
) -> Option<String> {
    match parent {
        Some(PlanNodeParent::Node(parent)) => emitted.get(&parent).cloned(),
        Some(PlanNodeParent::Branch(_)) | None => Some(plan_key.to_owned()),
    }
}

/// Reserved upstream keys and ordered rows for one projected plan copy.
///
/// # Public members
/// - `rows`: Ordered rows receiving the plan's node rows.
/// - `emitted`: Reserved row keys for nodes already projected beneath the plan.
///
/// # Private members
/// - `used`: Reserved upstream keys shared across every plan copy.
struct PlanRowSink<'a> {
    /// Ordered rows receiving the plan's node rows.
    rows: &'a mut Vec<PlanTreeRow>,
    /// Reserved upstream keys shared across every plan copy.
    used: &'a mut std::collections::HashSet<String>,
    /// Reserved row keys for nodes already projected beneath the plan.
    emitted: std::collections::BTreeMap<EntityId, String>,
}

impl PlanRowSink<'_> {
    /// Appends one node row after its structural parent is available.
    ///
    /// - `plan`: Durable plan containing the node.
    /// - `node`: Node being projected.
    /// - `plan_key`: Reserved tree key for this plan copy's header row.
    /// - `parent_id`: Structural tree parent row.
    /// - `semantics`: Shared views for this plan copy.
    fn node_row(
        &mut self,
        plan: &Plan,
        node: &PlanNode,
        plan_key: &str,
        parent_id: Option<String>,
        semantics: &Arc<SharedPlanViews>,
    ) {
        let id = super::common::unique_row_key(self.used, format!("{plan_key}:node:{}", node.id));
        self.emitted.entry(node.id).or_insert_with(|| id.clone());
        self.rows.push(PlanTreeRow {
            id,
            label: node_label(semantics, node),
            parent_id,
            selection: PlanSelection {
                plan: plan.id,
                node: Some(node.id),
            },
            semantics: Arc::clone(semantics),
        });
    }
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
