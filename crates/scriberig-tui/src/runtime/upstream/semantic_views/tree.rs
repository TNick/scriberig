//! Projects stable `ScribeRig` rows into reusable upstream tree items.

use rpi_tui::TreeItem;

use super::projection::TreeProjection;
use crate::state::TuiState;

/// Builds reusable tree items and details for the active semantic view.
///
/// - `state`: Current `ScribeRig` view and cursor.
///
/// Stable upstream items, the semantic selection, and supplementary details.
/// # Returns
/// A tree projection when the active view has ready semantic rows.
pub(super) fn projection(state: &TuiState) -> Option<TreeProjection> {
    let rows = semantic_rows(state)?;
    let details = match state.primary_view {
        crate::selection::PrimaryView::Conversation => super::details::conversation_details(state),
        crate::selection::PrimaryView::Session => super::details::session_details(state),
        _ => return None,
    };
    Some(TreeProjection {
        items: make_tree_items(&rows),
        selected_id: selected_tree_id(state, &rows),
        details,
    })
}

/// Projects current visible conversation or session rows to stable tree IDs.
///
/// - `state`: `ScribeRig` view state supplying the visible rows.
///
/// The ordered semantic rows with their source depth.
/// # Returns
/// Projected rows for supported conversation and session views.
fn semantic_rows(state: &TuiState) -> Option<Vec<SemanticTreeRow>> {
    match state.primary_view {
        crate::selection::PrimaryView::Conversation => Some(
            state
                .conversation
                .as_ref()?
                .rows
                .iter()
                .map(|row| SemanticTreeRow {
                    id: conversation_source_id(row.source),
                    label: format!("{:?} · {}", row.kind, bounded_label(&row.label)),
                    depth: row.depth,
                })
                .collect(),
        ),
        crate::selection::PrimaryView::Session => match &state.session {
            crate::inspector::LoadState::Ready(model) => Some(
                model
                    .rows
                    .iter()
                    .map(|row| SemanticTreeRow {
                        id: session_row_id(row.identity()),
                        label: format!(
                            "{:?} · {} · {}",
                            row.kind,
                            bounded_label(&row.label),
                            row.reference
                        ),
                        depth: row.depth,
                    })
                    .collect(),
            ),
            _ => None,
        },
        _ => None,
    }
}

/// Creates tree items from semantic preorder while preserving projected depth.
///
/// - `rows`: Ordered `ScribeRig` semantic rows.
///
/// Upstream tree items with stable IDs and nearest preceding parents.
/// # Returns
/// Tree items preserving the semantic preorder.
fn make_tree_items(rows: &[SemanticTreeRow]) -> Vec<TreeItem> {
    let mut ancestors = Vec::<(usize, String)>::new();
    rows.iter()
        .map(|row| {
            while ancestors
                .last()
                .is_some_and(|(depth, _)| *depth >= row.depth)
            {
                ancestors.pop();
            }
            let parent = ancestors.last().map(|(_, id)| id.clone());
            ancestors.push((row.depth, row.id.clone()));
            TreeItem {
                id: row.id.clone(),
                label: row.label.clone(),
                parent_id: parent,
            }
        })
        .collect()
}

/// Resolves current selection to a stable upstream item ID.
///
/// - `state`: `ScribeRig`'s selected row.
/// - `rows`: Stable row IDs in the active tree.
///
/// The matching ID, when the current row remains projected.
/// # Returns
/// Stable item ID when the selected row is visible.
fn selected_tree_id(state: &TuiState, rows: &[SemanticTreeRow]) -> Option<String> {
    let selected = match state.primary_view {
        crate::selection::PrimaryView::Conversation => {
            state.conversation_cursor.map(conversation_source_id)
        }
        crate::selection::PrimaryView::Session => state.session_cursor.map(session_row_id),
        _ => None,
    };

    // A retained semantic identity must never be replaced by an unrelated row
    // that happens to occupy its old flattened cursor after projection changes.
    if let Some(selected) = selected {
        return rows
            .iter()
            .find(|row| row.id == selected)
            .map(|row| row.id.clone());
    }

    rows.get(state.cursor).map(|row| row.id.clone())
}

/// Converts a typed stable session-row identity to its upstream tree key.
///
/// - `identity`: `ScribeRig` row identity retained across projection refreshes.
///
/// A namespaced upstream tree item ID.
/// # Returns
/// Stable item ID namespaced for session rows.
fn session_row_id(identity: crate::view::session::SessionRowIdentity) -> String {
    format!("session-row:{:?}:{}", identity.kind, identity.reference)
}

/// Converts a typed conversation source to a namespaced tree item ID.
///
/// - `source`: Stable `ScribeRig` source identity.
///
/// The stable ID used by the upstream tree.
/// # Returns
/// Stable item ID namespaced for conversation sources.
fn conversation_source_id(source: crate::view::conversation::ConversationSource) -> String {
    use crate::view::conversation::ConversationSource as Source;
    match source {
        Source::Session(id) => format!("session:{id}"),
        Source::Branch(id) => format!("branch:{id}"),
        Source::Activity(id) => format!("activity:{id}"),
        Source::BranchResult(id) => format!("branch-result:{id}"),
        Source::Plan(id) => format!("plan:{id}"),
        Source::PlanNode(id) => format!("plan-node:{id}"),
        Source::TransientActivity(id) => format!("transient:{id}"),
    }
}

/// Sanitizes and bounds a semantic label before it reaches `TreeList` rendering.
///
/// - `label`: Projected `ScribeRig` row label.
///
/// A terminal-safe label limited to 800 characters.
/// # Returns
/// Sanitized and bounded semantic label.
fn bounded_label(label: &str) -> String {
    crate::inspector::sanitize(label)
        .chars()
        .take(800)
        .collect()
}

/// Carries one semantic tree row between projections and the upstream widget.
pub(super) struct SemanticTreeRow {
    /// Namespaced stable `ScribeRig` source identity.
    pub(super) id: String,
    /// Bounded terminal-safe row label.
    label: String,
    /// `ScribeRig`-projected hierarchy depth.
    depth: usize,
}
