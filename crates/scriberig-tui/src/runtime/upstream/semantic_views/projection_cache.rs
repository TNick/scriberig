//! Caches expensive semantic tree projections between state changes.

use std::sync::Arc;

use super::projection::TreeProjection;
use super::{SemanticViewPanel, TuiState, plan, projection, tree};
use crate::selection::PrimaryView;

/// Distinguishes views without a semantic tree from cached tree projections.
enum CachedProjection {
    /// The active view is rendered by another component.
    NonTree,
    /// The active view has a reusable semantic tree.
    Tree(Arc<TreeProjection>),
}

/// One immutable projected tree keyed by the state revision that produced it.
///
/// # Public members
/// None.
///
/// # Private members
/// - `key`: Presentation revision and view represented by the cache.
/// - `projection`: Cached projection, including cached absence for non-tree views.
/// - `tree_synced`: Whether the upstream tree has received the projection.
/// - `details_rendered`: Whether the Markdown panel has received the detail text.
#[derive(Default)]
pub(super) struct ProjectionCache {
    /// Revision and view represented by the optional cached projection.
    key: Option<(u64, PrimaryView)>,
    /// Cached tree or a marker that the selected view has no semantic tree.
    projection: Option<CachedProjection>,
    /// Whether the upstream `TreeList` has received this projection.
    tree_synced: bool,
    /// Whether the upstream Markdown panel has received its detail text.
    details_rendered: bool,
}

impl ProjectionCache {
    /// Gets or builds the tree projection for the current presentation revision.
    ///
    /// # Parameters
    /// - `self`: Projection cache receiving or reusing the computed tree.
    /// - `state`: Presentation state supplying semantic rows and details.
    ///
    /// The cached projection, or `None` for views rendered by another component.
    ///
    /// # Returns
    /// The cached upstream projection when the selected view is tree-backed.
    pub(super) fn get(&mut self, state: &TuiState) -> Option<Arc<TreeProjection>> {
        let key = (state.render_revision, state.primary_view);
        if self.key != Some(key) {
            self.key = Some(key);
            self.projection = Some(
                build_projection(state).map_or(CachedProjection::NonTree, |projection| {
                    CachedProjection::Tree(Arc::new(projection))
                }),
            );
            self.tree_synced = false;
            self.details_rendered = false;
        } else if matches!(
            state.primary_view,
            PrimaryView::Conversation | PrimaryView::Session
        ) && let Some(CachedProjection::Tree(projection)) = self.projection.as_ref()
            && projection.selected_id != tree::selected_cursor_id(state, projection)
        {
            let Some(CachedProjection::Tree(projection)) = self.projection.as_mut() else {
                unreachable!();
            };
            let projection = Arc::make_mut(projection);
            tree::refresh_cursor_projection(state, projection);
            if state.primary_view == PrimaryView::Session {
                self.details_rendered = false;
            }
        }
        match self.projection.as_ref()? {
            CachedProjection::NonTree => None,
            CachedProjection::Tree(projection) => Some(Arc::clone(projection)),
        }
    }

    /// Reports whether the current projection needs to update the upstream tree.
    ///
    /// # Parameters
    /// - `self`: Projection cache tracking upstream tree synchronization.
    ///
    /// # Returns
    /// `true` when the current projection has not been synchronized.
    pub(super) fn needs_tree_sync(&self) -> bool {
        !self.tree_synced
    }

    /// Records that the current projection is installed in the upstream tree.
    ///
    /// # Parameters
    /// - `self`: Projection cache tracking upstream tree synchronization.
    pub(super) fn mark_tree_synced(&mut self) {
        self.tree_synced = true;
    }

    /// Reports whether the current details need to update the Markdown panel.
    ///
    /// # Parameters
    /// - `self`: Projection cache tracking Markdown detail rendering.
    ///
    /// # Returns
    /// `true` when the current detail has not been rendered.
    pub(super) fn needs_details_render(&self) -> bool {
        !self.details_rendered
    }

    /// Records that the current details are installed in the Markdown panel.
    ///
    /// # Parameters
    /// - `self`: Projection cache tracking Markdown detail rendering.
    pub(super) fn mark_details_rendered(&mut self) {
        self.details_rendered = true;
    }
}

/// Builds the active upstream tree projection for one semantic view.
///
/// - `state`: Presentation state supplying the selected view.
///
/// # Returns
/// A tree projection for conversation, session, or plan views.
fn build_projection(state: &TuiState) -> Option<TreeProjection> {
    match state.primary_view {
        PrimaryView::Conversation | PrimaryView::Session => tree::projection(state),
        PrimaryView::Plan => plan::projection(state).map(|(projection, _)| projection),
        _ => None,
    }
}

#[cfg(test)]
#[path = "projection_cache_tests.rs"]
mod tests;

impl SemanticViewPanel {
    /// Returns the cached projection, building it once for each state revision.
    ///
    /// # Parameters
    /// - `self`: Panel owning the cached tree projection.
    /// - `state`: Presentation state protected by the active runtime.
    ///
    /// # Returns
    /// The current upstream tree projection.
    pub(super) fn projection_for_state(&self, state: &TuiState) -> Option<Arc<TreeProjection>> {
        let Ok(mut cache) = self.projection_cache.lock() else {
            return build_projection(state).map(Arc::new);
        };
        cache.get(state)
    }

    /// Synchronizes the upstream tree only after its source revision changes.
    ///
    /// # Parameters
    /// - `self`: Panel retaining the upstream tree and its cache state.
    /// - `view`: Semantic view whose retained tree must be synchronized.
    /// - `projection`: Immutable semantic tree projection to install.
    ///
    /// # Returns
    /// `true` when the retained upstream tree is ready to render.
    pub(super) fn sync_tree_for_projection(
        &self,
        view: PrimaryView,
        projection: &TreeProjection,
    ) -> bool {
        let Ok(mut cache) = self.projection_cache.lock() else {
            return false;
        };
        let Ok(mut trees) = self.tree.lock() else {
            return false;
        };
        if !cache.needs_tree_sync() {
            let Some(tree) = trees.get(&view).and_then(Option::as_ref) else {
                return false;
            };
            if let Some(selected_id) = projection.selected_id.as_deref()
                && tree.tree.selected_id().as_deref() != Some(selected_id)
            {
                let _ = tree.tree.select(selected_id);
            }
            return true;
        }
        if projection::sync_tree_for_view(&mut trees, view, projection).is_none() {
            return false;
        }
        cache.mark_tree_synced();
        true
    }

    /// Updates detail Markdown only after its source revision changes.
    ///
    /// # Parameters
    /// - `self`: Panel owning the upstream Markdown renderer.
    /// - `projection`: Immutable semantic details to render.
    pub(super) fn render_details_for_projection(&self, projection: &TreeProjection) {
        let Ok(mut cache) = self.projection_cache.lock() else {
            return;
        };
        if cache.needs_details_render() {
            self.markdown.set_content(projection.details.clone());
            cache.mark_details_rendered();
        }
    }
}
