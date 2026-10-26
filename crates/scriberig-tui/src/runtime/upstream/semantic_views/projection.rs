//! Reuses one upstream tree across changing `ScribeRig` semantic projections.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::selection::PrimaryView;
use rpi_tui::{TreeItem, TreeList};

/// One reusable upstream tree projection for the active semantic view.
///
/// # Public members
/// None.
///
/// # Private members
/// - `items`: Stable tree items for the current semantic projection.
/// - `selected_id`: Stable upstream item ID selected by `ScribeRig`.
/// - `details`: Markdown supplementary details for the selected row.
#[derive(Clone)]
pub(super) struct TreeProjection {
    /// Stable tree items for the current semantic projection.
    pub(super) items: Vec<TreeItem>,
    /// Stable upstream item ID selected by `ScribeRig`.
    pub(super) selected_id: Option<String>,
    /// Markdown supplementary details for the selected row.
    pub(super) details: String,
}

/// Retains an upstream tree together with the semantic items it represents.
///
/// # Public members
/// None.
///
/// # Private members
/// - `tree`: Upstream tree component for the current item projection.
/// - `items`: Last item set used to create the tree.
pub(super) struct RetainedTree {
    /// Upstream tree component for the current item projection.
    pub(super) tree: TreeList,
    /// Last item set used to create the tree.
    items: Vec<TreeItem>,
}

/// Creates a tree that exposes every projected parent's children.
///
/// - `items`: Validated semantic items in display order.
///
/// An upstream tree with all current parent rows expanded.
/// # Returns
/// The initialized tree, or `None` when item validation fails.
pub(super) fn tree_from_items(items: &[TreeItem]) -> Option<TreeList> {
    let tree = TreeList::new(items.to_vec(), super::common::MAX_ROWS).ok()?;
    let parent_ids: HashSet<&str> = items
        .iter()
        .filter_map(|item| item.parent_id.as_deref())
        .collect();
    let parent_ids = parent_ids
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let _ = tree.set_expanded_many(&parent_ids, true);
    Some(tree)
}

/// Reuses one upstream tree and applies only changed semantic items.
///
/// - `slot`: Retained tree component for the panel.
/// - `projection`: Current semantic items, selection, and details.
///
/// The retained tree updated to the projection.
/// # Returns
/// `Some(())` when the tree is ready to render or handle input.
pub(super) fn sync_tree(
    slot: &mut Option<RetainedTree>,
    projection: &TreeProjection,
) -> Option<()> {
    if let Some(retained) = slot.as_mut() {
        if retained.items != projection.items {
            let revealed = revealed_parents(&retained.items, &projection.items);
            retained.tree.replace_items(projection.items.clone()).ok()?;
            let _ = retained.tree.set_expanded_many(&revealed, true);
            retained.items.clone_from(&projection.items);
        }
    } else {
        *slot = Some(RetainedTree {
            tree: tree_from_items(&projection.items)?,
            items: projection.items.clone(),
        });
    }
    let tree = &slot.as_ref()?.tree;
    let selected_id = projection
        .selected_id
        .as_deref()
        .or_else(|| projection.items.first().map(|item| item.id.as_str()));
    if let Some(selected_id) = selected_id
        && tree.selected_id().as_deref() != Some(selected_id)
    {
        let _ = tree.select(selected_id);
    }
    Some(())
}

/// Synchronizes one retained tree without sharing state across semantic views.
///
/// - `trees`: Per-view retained tree slots.
/// - `view`: Semantic view whose upstream tree is being updated.
/// - `projection`: Current rows, stable selection, and details for that view.
///
/// # Returns
/// `Some(())` when the view tree is ready, otherwise `None` for invalid rows.
pub(super) fn sync_tree_for_view(
    trees: &mut HashMap<PrimaryView, Option<RetainedTree>>,
    view: PrimaryView,
    projection: &TreeProjection,
) -> Option<()> {
    sync_tree(trees.entry(view).or_default(), projection)
}

/// Lists the rows whose children a semantic projection has just materialized.
///
/// - `previous`: Item set the retained tree projected before this change.
/// - `items`: Item set the retained tree projects now.
///
/// `ScribeRig` owns semantic collapse by omitting a collapsed row's children
/// from the projected item set, so children appearing under a row that had none
/// is an expansion that row's own semantic reducer requested. Upstream
/// `TreeList::replace_items` expands only IDs that are new to the item set, so
/// an already projected row would keep its stale collapsed state and hide the
/// rows the reducer just produced. A row that already had children keeps the
/// tree's own expansion state, which is how collapse bound to a stable identity
/// survives a refresh.
///
/// Parents whose children this projection has just materialized.
/// # Returns
/// Item IDs to expand, deduplicated and sorted by item ID.
fn revealed_parents(previous: &[TreeItem], items: &[TreeItem]) -> Vec<String> {
    let parented_before: HashSet<&str> = previous
        .iter()
        .filter_map(|item| item.parent_id.as_deref())
        .collect();
    let mut revealed = HashSet::new();
    for item in items {
        if let Some(parent) = item.parent_id.as_deref()
            && !parented_before.contains(parent)
        {
            revealed.insert(parent);
        }
    }
    // Expansion order is deterministic so a refresh that reveals several rows
    // leaves the same visible sequence on every run.
    let mut revealed: Vec<String> = revealed.into_iter().map(str::to_owned).collect();
    revealed.sort();
    revealed
}
