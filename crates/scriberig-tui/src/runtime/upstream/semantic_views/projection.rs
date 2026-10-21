//! Reuses one upstream tree across changing `ScribeRig` semantic projections.

use std::collections::HashSet;

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
    for item in items {
        if parent_ids.contains(item.id.as_str()) {
            let _ = tree.set_expanded(&item.id, true);
        }
    }
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
            retained.tree.replace_items(projection.items.clone()).ok()?;
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
