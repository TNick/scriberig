//! Defines `ScribeRig` command palette rows; upstream owns filtering and navigation.

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

/// Available palette rows with stable `ScribeRig` action identities.
///
/// # Public members
/// - `local_actions`: Local actions and capability-filtered semantic actions.
/// - `semantic_actions`: Additional capability-derived actions.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaletteState {
    /// Available rows.
    pub local_actions: Vec<PaletteItem>,
    /// Future capability-derived rows.
    pub semantic_actions: Vec<PaletteItem>,
}

impl Default for PaletteState {
    /// Creates the initial local palette.
    ///
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self {
            local_actions: vec![
                item("help", "Open help"),
                item("refresh", "Refresh"),
                item("back", "Go back"),
                item("quit", "Quit"),
                item("conversation", "View conversation"),
                item("overview", "View overview"),
                item("explorer", "Explore semantic entities"),
                item("find", "Find semantic entities"),
                item("history", "View history"),
                item("diff", "View diff"),
                item("projection", "View projection"),
                item("edit", "Edit source projection"),
                item("project-agent", "Agent change selected entity"),
                item("changes", "View changes/validation"),
                item("review-candidate", "Review candidate"),
                item("confirm-review", "Confirm reviewed change"),
                item("session", "View session"),
                item("plans", "View plans and dependencies"),
                item("model-picker", "Select model"),
                item("thinking-picker", "Select thinking level"),
                item("theme-picker", "Select terminal theme"),
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
                "session" | "plans" => available("session_branch_tree"),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserves stable command IDs and labels without local filtering state.
    #[test]
    fn palette_items_preserve_command_identity() {
        let palette = PaletteState::default();
        let refresh = palette
            .local_actions
            .iter()
            .find(|item| item.id == "refresh")
            .expect("refresh command");
        assert_eq!(refresh.label, "Refresh");
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
