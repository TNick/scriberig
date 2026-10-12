//! Implements local command palette filtering and selection.

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
    /// The value produced by the declaration.
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
                item("changes", "View changes/validation"),
                item("session", "View session"),
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
    /// The value produced by the declaration.
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
/// # Parameters
/// - `id`: Stable action identifier.
/// - `label`: Human-readable action label.
///
/// # Returns
/// A palette item.
fn item(id: &str, label: &str) -> PaletteItem {
    item_with_source(id, label, ActionSource::Local)
}

/// Creates a palette item with an explicit action source.
///
/// # Parameters
/// - `id`: Stable action identifier.
/// - `label`: Human-readable action label.
/// - `source`: Action source classification.
///
/// # Returns
/// A palette item.
fn item_with_source(id: &str, label: &str, source: ActionSource) -> PaletteItem {
    PaletteItem {
        id: id.to_owned(),
        label: label.to_owned(),
        source,
    }
}

/// Filters local and semantic rows by case-insensitive substring matching.
#[must_use]
/// - `items`: Argument named `items`.
/// - `query`: Argument named `query`.
///
/// # Returns
/// The value produced by the declaration.
pub fn filter(items: &[PaletteItem], query: &str) -> Vec<PaletteItem> {
    let query = query.to_lowercase();
    items
        .iter()
        .filter(|item| {
            item.label.to_lowercase().contains(&query) || item.id.to_lowercase().contains(&query)
        })
        .cloned()
        .collect()
}

impl PaletteState {
    /// Returns the currently visible filtered rows.
    #[must_use]
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn filtered(&self) -> Vec<PaletteItem> {
        let mut items = self.local_actions.clone();
        items.extend(self.semantic_actions.clone());
        filter(&items, &self.query)
    }
    /// Changes the filter and clamps the selection.
    /// - `query`: Argument named `query`.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected = self.selected.min(self.filtered().len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    /// Implements `filters_local_actions_case_insensitively`.
    fn filters_local_actions_case_insensitively() {
        let state = PaletteState::default();
        assert_eq!(
            state
                .filtered()
                .iter()
                .filter(|item| item.label == "Open help")
                .count(),
            1
        );
        assert_eq!(filter(&state.local_actions, "REF")[0].id, "refresh");
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
}
