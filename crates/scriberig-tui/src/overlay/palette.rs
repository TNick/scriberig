//! Implements local command palette filtering and selection.

use scriberig_tui_kit::{SelectionOption, StableSelector};

/// A palette action source, ready for future semantic capability actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionSource {
    /// Local presentation command.
    Local,
    /// Semantic contextual command.
    Semantic,
}

/// One command palette row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaletteItem {
    /// Stable action identifier.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Origin of the action.
    pub source: ActionSource,
}

/// Palette state with local actions and a future semantic source.
///
/// # Public members
/// - `query`: Current user filter.
/// - `local_actions`: Local actions and capability-filtered semantic actions.
/// - `semantic_actions`: Additional capability-derived actions.
/// - `selected`: Disposable cursor index into filtered rows.
/// - [`PaletteState::selector`]: Creates an upstream selector with stable IDs.
/// - [`PaletteState::filtered`]: Returns the visible filtered rows.
/// - [`PaletteState::set_query`]: Replaces the filter.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaletteState {
    /// User filter.
    pub query: String,
    /// Available rows.
    pub local_actions: Vec<PaletteItem>,
    /// Future capability-derived rows.
    pub semantic_actions: Vec<PaletteItem>,
    /// Selected filtered row.
    pub selected: usize,
}

impl Default for PaletteState {
    /// Creates the initial local palette.
    ///
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self {
            query: String::new(),
            local_actions: vec![
                item("help", "Open help"),
                item("refresh", "Refresh"),
                item("back", "Go back"),
                item("quit", "Quit"),
                item("overview", "View overview"),
                item("history", "View history"),
                item("diff", "View diff"),
                item("projection", "View projection"),
                item("edit", "Edit source projection"),
                item("project-agent", "Agent change selected entity"),
                item("changes", "View changes/validation"),
                item("review-candidate", "Review candidate"),
                item("session", "View session"),
                item("model-picker", "Select model"),
                item("thinking-picker", "Select thinking level"),
                item(
                    "toggle-tool-output",
                    "Cycle tool result display: hidden/summary/full",
                ),
                item(
                    "toggle-tool-invocation",
                    "Toggle tool call detail: simple/detailed",
                ),
                item(
                    "toggle-prompt",
                    "Toggle captured model request: hidden/shown",
                ),
                item(
                    "toggle-reasoning",
                    "Toggle safe reasoning summaries and status: hidden/shown",
                ),
                item("start-agent", "Start agent session"),
                item("continue-agent", "Continue selected branch"),
                item("operations", "View operations"),
            ],
            semantic_actions: Vec::new(),
            selected: 0,
        }
    }
}

impl PaletteState {
    /// Creates a palette whose semantic rows reflect application capabilities.
    #[must_use]
    /// - `capabilities`: Argument named `capabilities`.
    ///
    /// # Returns
    /// The operation result.
    pub fn with_capabilities(capabilities: &[scriberig_app::RuntimeCapability]) -> Self {
        let mut palette = Self::default();
        if !capabilities.is_empty() {
            let available = |id: &str| {
                capabilities
                    .iter()
                    .any(|capability| capability.available && capability.id.as_str() == id)
            };
            palette.local_actions.retain(|item| match item.id.as_str() {
                "history" => available("history"),
                "diff" => available("diff"),
                "projection" => available("projection_render"),
                "edit" | "project-agent" => available("projection_edit_candidate"),
                "changes" => available("change_review"),
                "session" => available("session_branch_tree"),
                "start-agent" => available("agent_start"),
                "continue-agent" => available("agent_continue"),
                _ => true,
            });
        }
        if capabilities
            .iter()
            .any(|capability| capability.available && capability.id.as_str() == "inspect_entity")
        {
            palette.semantic_actions.push(item_with_source(
                "inspect_entity",
                "Inspect entity",
                ActionSource::Semantic,
            ));
        }
        palette
    }
}

/// Creates a local palette item.
///
/// - `id`: Stable action identifier.
/// - `label`: Human-readable action label.
///
/// A palette item.
/// # Returns
/// The operation result.
fn item(id: &str, label: &str) -> PaletteItem {
    item_with_source(id, label, ActionSource::Local)
}

/// Creates a palette item with an explicit action source.
///
/// - `id`: Stable action identifier.
/// - `label`: Human-readable action label.
/// - `source`: Action source classification.
///
/// A palette item.
/// # Returns
/// The operation result.
fn item_with_source(id: &str, label: &str, source: ActionSource) -> PaletteItem {
    PaletteItem {
        id: id.to_owned(),
        label: label.to_owned(),
        source,
    }
}

impl PaletteState {
    /// Returns the currently visible filtered rows.
    #[must_use]
    ///
    /// # Returns
    /// The operation result.
    pub fn filtered(&self) -> Vec<PaletteItem> {
        let items = self.items();
        let selector =
            StableSelector::with_filter(&self.options(), items.len().max(1), None, &self.query);
        selector
            .filtered_options()
            .iter()
            .filter_map(|option| items.iter().find(|item| item.id == option.id))
            .cloned()
            .collect()
    }
    /// Changes the filter and clamps the selection.
    /// - `query`: Argument named `query`.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected = self.selected.min(self.filtered().len().saturating_sub(1));
    }

    /// Builds an upstream-backed selector while preserving action identities.
    ///
    /// # Parameters
    /// - `max_visible`: Maximum number of visible command rows.
    ///
    /// # Returns
    /// A selector whose selected value is a stable command ID.
    #[must_use]
    pub fn selector(&self, max_visible: usize) -> StableSelector {
        let filtered = self.filtered();
        let selected_id = filtered.get(self.selected).map(|item| item.id.clone());
        StableSelector::with_filter(
            &self.options(),
            max_visible,
            selected_id.as_deref(),
            &self.query,
        )
    }

    /// Returns all currently available palette actions.
    ///
    /// # Returns
    /// Local and capability-filtered semantic actions.
    fn items(&self) -> Vec<PaletteItem> {
        self.local_actions
            .iter()
            .chain(&self.semantic_actions)
            .cloned()
            .collect()
    }

    /// Converts available actions to terminal-kit selector options.
    ///
    /// # Returns
    /// Options preserving each stable action ID.
    fn options(&self) -> Vec<SelectionOption> {
        self.items()
            .into_iter()
            .map(|item| SelectionOption {
                id: item.id,
                label: item.label,
                description: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    /// Implements `filters_local_actions_case_insensitively`.
    fn filters_local_actions_through_the_selector() {
        let mut state = PaletteState::default();
        state.set_query("REF".to_owned());
        assert!(state.filtered().iter().any(|item| item.id == "refresh"));
    }

    /// Verifies inspect is advertised only by an available runtime capability.
    #[test]
    fn semantic_actions_follow_capability_availability() {
        let capability = scriberig_app::RuntimeCapability::new(
            scriberig_app::CapabilityId::new("inspect_entity").expect("id"),
            "inspect",
            true,
        );
        let palette = PaletteState::with_capabilities(&[capability]);
        assert!(
            palette
                .semantic_actions
                .iter()
                .any(|item| item.id == "inspect_entity")
        );
        assert_eq!(PaletteState::default().semantic_actions.len(), 0);
    }

    /// Verifies agent actions require their own advertised capabilities.
    #[test]
    fn agent_actions_follow_specific_capabilities() {
        let session = scriberig_app::RuntimeCapability::new(
            scriberig_app::CapabilityId::new("session_branch_tree").expect("id"),
            "session",
            true,
        );
        let start = scriberig_app::RuntimeCapability::new(
            scriberig_app::CapabilityId::new("agent_start").expect("id"),
            "start",
            true,
        );
        let palette = PaletteState::with_capabilities(&[session, start]);
        let ids = palette
            .local_actions
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"session"));
        assert!(ids.contains(&"start-agent"));
        assert!(!ids.contains(&"continue-agent"));
    }

    /// Verifies the production palette selector returns a stable action ID.
    #[test]
    fn selector_preserves_action_identity_after_filtering() {
        let mut palette = PaletteState::default();
        palette.set_query("refresh".to_owned());

        let selector = palette.selector(10);

        assert_eq!(selector.selected_id(), Some("refresh"));
        assert!(
            selector
                .render(40)
                .iter()
                .any(|line| line.as_str().contains("Refresh"))
        );
    }
}
