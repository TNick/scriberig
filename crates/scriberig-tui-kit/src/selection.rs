//! Adapts upstream fuzzy selection while retaining stable caller IDs.

use rpi_tui::{Component, SelectItem, SelectList, fuzzy_filter};

use crate::text::ExternalText;

/// Provides an item with a stable caller-owned identifier.
///
/// # Public members
/// - `id`: Stable identity returned when selected.
/// - `label`: Human-readable label shown in the terminal.
/// - `description`: Optional explanatory text.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionOption {
    /// Stable identity returned when selected.
    pub id: String,
    /// Human-readable label shown in the terminal.
    pub label: String,
    /// Optional explanatory text shown with the item.
    pub description: Option<String>,
}

/// Provides a filtered selection list that returns stable caller identities.
///
/// # Public members
/// - [`StableSelector::new`]: Creates a selector over caller items.
/// - [`StableSelector::with_filter`]: Creates a filtered selector.
/// - [`StableSelector::with_selection`]: Restores a stable selected ID.
/// - [`StableSelector::move_up`]: Selects the previous item.
/// - [`StableSelector::move_down`]: Selects the next item.
/// - [`StableSelector::selected_id`]: Returns the selected stable identity.
/// - [`StableSelector::filtered_options`]: Returns options in fuzzy match order.
/// - [`StableSelector::render`]: Renders safe plain lines.
///
/// # Private members
/// - `filtered_options`: Retains identities after upstream fuzzy filtering.
/// - `component`: Holds the upstream selection component.
/// - `selected_index`: Tracks the disposable cursor in filtered rows.
pub struct StableSelector {
    filtered_options: Vec<SelectionOption>,
    component: SelectList,
    selected_index: usize,
}

impl StableSelector {
    /// Creates a selector from caller-owned items.
    ///
    /// # Parameters
    /// - `options`: Items with caller-owned stable identities.
    /// - `max_visible`: Maximum number of rows shown at once.
    ///
    /// # Returns
    /// A selector initially focused on the first item.
    #[must_use]
    pub fn new(options: &[SelectionOption], max_visible: usize) -> Self {
        Self::with_filter(options, max_visible, None, "")
    }

    /// Creates a selector with fuzzy filtering and an optional stable identity.
    ///
    /// # Parameters
    /// - `options`: Items with caller-owned stable identities.
    /// - `max_visible`: Maximum number of rows shown at once.
    /// - `selected_id`: Stable identity to retain when it matches the filter.
    /// - `query`: User text passed to upstream fuzzy filtering.
    ///
    /// # Returns
    /// A selector focused on the requested item or the first match.
    #[must_use]
    pub fn with_filter(
        options: &[SelectionOption],
        max_visible: usize,
        selected_id: Option<&str>,
        query: &str,
    ) -> Self {
        let items = upstream_items(options);
        let filtered_items = fuzzy_filter(&items, query, SelectItem::search_key);
        let filtered_options = filtered_items
            .iter()
            .filter_map(|item| options.iter().find(|option| option.id == item.value))
            .cloned()
            .collect::<Vec<_>>();
        let selected_index = selected_id
            .and_then(|id| filtered_options.iter().position(|option| option.id == id))
            .unwrap_or(0);
        let component = SelectList::new(items, max_visible);
        component.set_filter(query);
        component.set_selected_index(selected_index);
        Self {
            filtered_options,
            component,
            selected_index,
        }
    }

    /// Creates a selector and restores a caller-owned stable identity.
    ///
    /// # Parameters
    /// - `options`: Items with caller-owned stable identities.
    /// - `max_visible`: Maximum number of rows shown at once.
    /// - `selected_id`: Stable identity to select when present.
    ///
    /// # Returns
    /// A selector focused on the requested item or the first item.
    #[must_use]
    pub fn with_selection(
        options: &[SelectionOption],
        max_visible: usize,
        selected_id: Option<&str>,
    ) -> Self {
        Self::with_filter(options, max_visible, selected_id, "")
    }

    /// Selects the previous item, keeping selection inside the filtered list.
    pub fn move_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
        self.component.set_selected_index(self.selected_index);
    }

    /// Selects the next item, keeping selection inside the filtered list.
    pub fn move_down(&mut self) {
        if !self.filtered_options.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.filtered_options.len() - 1);
            self.component.set_selected_index(self.selected_index);
        }
    }

    /// Returns the stable ID of the selected filtered item, if one exists.
    ///
    /// # Returns
    /// The caller-owned identity for the currently selected item.
    #[must_use]
    pub fn selected_id(&self) -> Option<&str> {
        self.filtered_options
            .get(self.selected_index)
            .map(|option| option.id.as_str())
    }

    /// Returns caller options in upstream fuzzy-match order.
    ///
    /// # Returns
    /// The filtered items, retaining stable caller identities.
    #[must_use]
    pub fn filtered_options(&self) -> &[SelectionOption] {
        &self.filtered_options
    }

    /// Renders selection rows as safe plain text.
    ///
    /// # Parameters
    /// - `width`: Maximum rendering width in terminal columns.
    ///
    /// # Returns
    /// Sanitized text lines without upstream ANSI styling.
    #[must_use]
    pub fn render(&self, width: usize) -> Vec<ExternalText> {
        self.component
            .render(width)
            .iter()
            .map(|line| ExternalText::new(&rpi_tui::strip_ansi(line)))
            .collect()
    }
}

/// Converts caller options to sanitized upstream items.
///
/// # Parameters
/// - `options`: Caller-owned selection options.
///
/// # Returns
/// Upstream items with stable values and bounded display strings.
fn upstream_items(options: &[SelectionOption]) -> Vec<SelectItem> {
    options
        .iter()
        .map(|option| {
            let id = ExternalText::new(&option.id);
            let label = ExternalText::new(&option.label);
            let search_text = format!("{} {}", id.as_str(), label.as_str());
            let mut item =
                SelectItem::new(id.as_str(), label.as_str()).with_search_text(&search_text);
            if let Some(description) = &option.description {
                let description = ExternalText::new(description);
                item = item.with_description(description.as_str());
            }
            item
        })
        .collect()
}
