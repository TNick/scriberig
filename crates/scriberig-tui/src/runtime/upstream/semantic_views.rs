//! Renders `ScribeRig` semantic projections through upstream tree and Markdown components.

use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use rpi_tui::keybindings::keys;
use rpi_tui::{Component, InputEvent, Keybindings, Markdown, SelectList};

use crate::state::TuiState;
use projection::RetainedTree;
use projection_cache::ProjectionCache;

#[path = "semantic_views/common.rs"]
mod common;
#[path = "semantic_views/content.rs"]
mod content;
#[path = "semantic_views/details.rs"]
mod details;
#[path = "semantic_views/model_views.rs"]
mod model_views;
#[path = "semantic_views/plan.rs"]
mod plan;
#[path = "semantic_views/projection.rs"]
mod projection;
#[path = "semantic_views/projection_cache.rs"]
mod projection_cache;
#[path = "semantic_views/runtime_views.rs"]
mod runtime_views;
#[path = "semantic_views/selection.rs"]
mod selection;
#[path = "semantic_views/tree.rs"]
mod tree;

pub(super) use plan::{
    reconcile_cursor as reconcile_plan_cursor, retain_cursor_identity as retain_plan_cursor,
};
pub(super) use selection::{reconcile_cursor, retain_cursor_identity};

/// Adapts selected `ScribeRig` semantic views to upstream tree and Markdown APIs.
///
/// # Public members
/// None.
///
/// # Private members
/// - `state`: Shared `ScribeRig` navigation and semantic view projections.
/// - `markdown`: Upstream Markdown renderer for supplementary details.
/// - `tree`: Upstream tree of the already-filtered visible `ScribeRig` rows.
/// - `projection_cache`: Last semantic projection keyed by presentation revision.
/// - `select_list`: Upstream list of stable non-tree semantic references.
/// - `keybindings`: Upstream keybinding manager shared with the active runtime.
/// - `input_sender`: Channel for `ScribeRig` navigation intents.
pub(super) struct SemanticViewPanel {
    /// Shared `ScribeRig` navigation and semantic view projections.
    state: Arc<Mutex<TuiState>>,
    /// Upstream Markdown renderer for supplementary details.
    markdown: Markdown,
    /// Upstream tree retained independently for each tree-backed semantic view.
    tree: Mutex<HashMap<crate::selection::PrimaryView, Option<RetainedTree>>>,
    /// Cached semantic rows and details reused between state updates.
    projection_cache: Mutex<ProjectionCache>,
    /// Upstream selectable list for stable non-tree semantic rows.
    select_list: Mutex<Option<SelectList>>,
    /// Upstream keybinding manager shared with the active runtime.
    keybindings: Arc<Keybindings>,
    /// Channel carrying `ScribeRig` navigation intents to its reducer.
    input_sender: Sender<super::ComposerInput>,
}

impl SemanticViewPanel {
    /// Creates the semantic projection component.
    ///
    /// - `state`: Shared TUI state supplying the active semantic view.
    /// - `input_sender`: Channel for forwarding `ScribeRig` navigation intents.
    /// - `keybindings`: Upstream keybinding manager shared with the runtime.
    ///
    /// # Returns
    /// A component rendered by upstream semantic-view components.
    pub(super) fn new(
        state: Arc<Mutex<TuiState>>,
        input_sender: Sender<super::ComposerInput>,
        keybindings: Arc<Keybindings>,
    ) -> Self {
        Self {
            state,
            markdown: Markdown::new("# Conversation\n\nType a message to start.", 1, 0),
            tree: Mutex::new(HashMap::new()),
            projection_cache: Mutex::new(ProjectionCache::default()),
            select_list: Mutex::new(None),
            keybindings,
            input_sender,
        }
    }
}

impl Component for SemanticViewPanel {
    /// Renders conversation/session rows with `TreeList` and other details with Markdown.
    ///
    /// - `width`: Available render width in terminal columns.
    /// - `view`: Semantic tree whose rows should be rendered.
    ///
    /// # Returns
    /// Rendered lines from the active upstream component.
    fn render(&self, width: usize) -> Vec<String> {
        let projection = self.state.lock().ok().and_then(|state| {
            let view = state.primary_view;
            self.projection_for_state(&state)
                .map(|projection| (view, projection))
        });
        if let Some((view, projection)) = projection {
            self.render_details_for_projection(&projection);
            let mut rows = if self.sync_tree_for_projection(view, &projection) {
                self.render_tree(view, width)
            } else {
                Vec::new()
            };
            rows.extend(self.markdown.render(width));
            return rows;
        }
        let Ok(state) = self.state.lock() else {
            return self.markdown.render(width);
        };
        if let Some((list, details)) = selection::semantic_list(&state) {
            if let Ok(mut current) = self.select_list.lock() {
                *current = Some(list);
            }
            self.markdown.set_content(details);
            let mut rows = self
                .select_list
                .lock()
                .ok()
                .and_then(|list| list.as_ref().map(|list| list.render(width)))
                .unwrap_or_default();
            rows.extend(self.markdown.render(width));
            return rows;
        }
        let content = content::render_view(&state);
        if self.markdown.get_content() != content {
            self.markdown.set_content(content);
        }
        if let Ok(mut list) = self.select_list.lock() {
            *list = None;
        }
        self.markdown.render(width)
    }

    /// Converts focused upstream navigation keys into `ScribeRig` intents.
    ///
    /// - `event`: Normalized event delivered by the upstream focus router.
    ///
    /// # Returns
    /// `true` when the event requests a `ScribeRig` navigation action.
    fn handle_event(&self, event: &InputEvent) -> bool {
        let InputEvent::Key(key) = event else {
            return false;
        };
        if self.keybindings.matches(key, super::controls::BROWSE) {
            let _ = self
                .input_sender
                .send(super::ComposerInput::TogglePresentationFocus(false));
            return true;
        }
        let direction = if self.keybindings.matches(key, keys::SELECT_UP) {
            Some(crate::action::Direction::Previous)
        } else if self.keybindings.matches(key, keys::SELECT_DOWN) {
            Some(crate::action::Direction::Next)
        } else {
            None
        };
        if self.keybindings.matches(key, keys::SELECT_CANCEL) {
            let _ = self
                .input_sender
                .send(super::ComposerInput::TogglePresentationFocus(false));
            return true;
        }
        if let Some(direction) = direction {
            let tree_view = self.state.lock().is_ok_and(|state| {
                matches!(
                    state.primary_view,
                    crate::selection::PrimaryView::Conversation
                        | crate::selection::PrimaryView::Session
                        | crate::selection::PrimaryView::Plan
                )
            });
            if tree_view {
                return self.handle_tree_movement(event, direction);
            }
            let list_view = self
                .state
                .lock()
                .is_ok_and(|state| selection::supports(&state));
            if list_view {
                return self.handle_list_movement(event, direction);
            }
            return self
                .input_sender
                .send(super::ComposerInput::PresentationAction(Box::new(
                    crate::action::Action::CursorMoved(direction),
                )))
                .is_ok();
        }
        if self.keybindings.matches(key, keys::CURSOR_LEFT)
            || self.keybindings.matches(key, keys::CURSOR_RIGHT)
        {
            let Ok(state) = self.state.lock() else {
                return false;
            };
            if state.primary_view != crate::selection::PrimaryView::Conversation
                || !state.conversation_browsing
            {
                return false;
            }
            drop(state);
            return self
                .input_sender
                .send(super::ComposerInput::PresentationAction(Box::new(
                    crate::action::Action::ConversationToggleExpansion,
                )))
                .is_ok();
        }
        if self.keybindings.matches(key, keys::SELECT_CONFIRM) {
            return self
                .input_sender
                .send(super::ComposerInput::PresentationAction(Box::new(
                    crate::action::Action::Activated,
                )))
                .is_ok();
        }
        false
    }

    /// Invalidates upstream Markdown's cached frame.
    fn invalidate(&self) {
        self.markdown.invalidate();
        if let Ok(trees) = self.tree.lock() {
            for tree in trees.values().flatten() {
                tree.tree.invalidate();
            }
        }
        if let Ok(list) = self.select_list.lock()
            && let Some(list) = list.as_ref()
        {
            list.invalidate();
        }
    }

    /// Supports upstream layout-node inspection.
    ///
    /// # Returns
    /// A type-erased reference to this component.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SemanticViewPanel {
    /// Renders the already-synchronized upstream semantic tree.
    ///
    /// - `view`: Semantic tree whose rows should be rendered.
    /// - `width`: Available render width in terminal columns.
    ///
    /// # Returns
    /// Rendered tree rows, or no rows when the tree is unavailable.
    fn render_tree(&self, view: crate::selection::PrimaryView, width: usize) -> Vec<String> {
        let Ok(trees) = self.tree.lock() else {
            return Vec::new();
        };
        trees
            .get(&view)
            .and_then(Option::as_ref)
            .map(|tree| tree.tree.render(width))
            .unwrap_or_default()
    }

    /// Moves the upstream selectable list and forwards `ScribeRig` cursor intent.
    ///
    /// - `event`: Upstream key event to apply to the list.
    /// - `direction`: Semantic movement direction selected by upstream keys.
    ///
    /// # Returns
    /// `true` when the active semantic list handles the key.
    fn handle_list_movement(
        &self,
        event: &InputEvent,
        direction: crate::action::Direction,
    ) -> bool {
        let Ok(state) = self.state.lock() else {
            return false;
        };
        let Some((list, rows)) = selection::list_from_state(&state) else {
            drop(state);
            return self.send_cursor_action(direction);
        };
        let before = list.get_selected_item().map(|item| item.value);
        let _ = list.handle_event(event);
        let after = list.get_selected_item().map(|item| item.value);
        let old_index = before.and_then(|id| rows.iter().position(|row| row.id == id));
        let new_index = after.and_then(|id| rows.iter().position(|row| row.id == id));
        drop(state);
        if let (Some(old), Some(new)) = (old_index, new_index)
            && old != new
        {
            let actual_direction = if new < old {
                crate::action::Direction::Previous
            } else {
                crate::action::Direction::Next
            };
            return self
                .input_sender
                .send(super::ComposerInput::PresentationAction(Box::new(
                    crate::action::Action::CursorMoved(actual_direction),
                )))
                .is_ok();
        }
        true
    }

    /// Moves the upstream tree cursor and forwards the matching `ScribeRig` action.
    ///
    /// - `event`: Upstream key event to apply to the tree.
    /// - `direction`: Semantic movement direction selected by upstream keys.
    ///
    /// # Returns
    /// `true` when the active semantic tree handles the key.
    fn handle_tree_movement(
        &self,
        event: &InputEvent,
        direction: crate::action::Direction,
    ) -> bool {
        let active = self.state.lock().ok().and_then(|state| {
            let view = state.primary_view;
            self.projection_for_state(&state)
                .map(|projection| (view, projection))
        });
        let Some((view, projection)) = active else {
            return self.send_cursor_action(direction);
        };
        if !self.sync_tree_for_projection(view, &projection) {
            return self.send_cursor_action(direction);
        }
        let Ok(trees) = self.tree.lock() else {
            return false;
        };
        let Some(tree) = trees.get(&view).and_then(Option::as_ref) else {
            drop(trees);
            return self.send_cursor_action(direction);
        };
        let before = tree.tree.selected_id();
        let _ = tree.tree.handle_event(event);
        let after = tree.tree.selected_id();
        drop(trees);
        let old_index =
            before.and_then(|id| projection.items.iter().position(|item| item.id == id));
        let new_index = after.and_then(|id| projection.items.iter().position(|item| item.id == id));
        if let (Some(old), Some(new)) = (old_index, new_index)
            && old != new
        {
            let actual_direction = if new < old {
                crate::action::Direction::Previous
            } else {
                crate::action::Direction::Next
            };
            return self
                .input_sender
                .send(super::ComposerInput::PresentationAction(Box::new(
                    crate::action::Action::CursorMoved(actual_direction),
                )))
                .is_ok();
        }
        true
    }

    /// Sends the current non-tree cursor movement action.
    ///
    /// - `direction`: Semantic movement direction selected by upstream keys.
    ///
    /// # Returns
    /// `true` when the action was queued.
    fn send_cursor_action(&self, direction: crate::action::Direction) -> bool {
        self.input_sender
            .send(super::ComposerInput::PresentationAction(Box::new(
                crate::action::Action::CursorMoved(direction),
            )))
            .is_ok()
    }
}
