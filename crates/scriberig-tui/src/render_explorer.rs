//! Renders the semantic explorer viewport and its selected row.

use crate::state::TuiState;

/// Formats the bounded semantic explorer model.
///
/// - `state`: Cached presentation state.
/// - `super`: Parent module visibility marker.
///
/// Text representing the explorer pane.
/// # Returns
/// The operation result.
pub(super) fn explorer_text(state: &TuiState) -> String {
    let Some(root) = state.explorer.root.as_ref() else {
        return "Explorer\n  No semantic root entered".to_owned();
    };
    let options = std::iter::once(root)
        .chain(state.explorer.rows.iter())
        .map(|row| scriberig_tui_kit::SelectionOption {
            id: selection_id(row),
            label: row.kind.as_deref().map_or_else(
                || row.label.clone(),
                |kind| format!("{} [{kind}]", row.label),
            ),
            description: Some(format!(
                "{} · revision {} · snapshot {}",
                row.selection.entity,
                row.selection
                    .revision
                    .map_or_else(|| "none".to_owned(), |value| value.to_string()),
                row.selection
                    .snapshot
                    .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            )),
        })
        .collect::<Vec<_>>();
    let selected_id = options.get(state.cursor).map(|option| option.id.as_str());
    let selector = scriberig_tui_kit::StableSelector::with_selection(
        &options,
        options.len().max(1),
        selected_id,
    );
    let mut rows = selector
        .render(4096)
        .into_iter()
        .map(|line| line.as_str().to_owned())
        .collect::<Vec<_>>();
    if state.explorer.next.is_some() {
        rows.push("  … more entities; press Enter on the root".to_owned());
    }
    format!("Explorer\n{}", rows.join("\n"))
}

/// Creates a stable caller ID from an exact semantic selection.
///
/// - `row`: Explorer row whose identity is retained by the selector.
///
/// An entity, revision, and snapshot key suitable for display selection.
/// # Returns
/// The stable semantic identity.
fn selection_id(row: &crate::explorer::ExplorerRow) -> String {
    format!(
        "{}:{}:{}",
        row.selection.entity,
        row.selection
            .revision
            .map_or_else(|| "none".to_owned(), |value| value.to_string()),
        row.selection
            .snapshot
            .map_or_else(|| "none".to_owned(), |value| value.to_string()),
    )
}

/// Computes the explorer text offset needed to keep the cursor in view.
///
/// - `state`: Current TUI presentation state.
/// - `height`: Outer explorer pane height.
/// - `super`: Parent module visibility marker.
///
/// The number of text rows to skip from the top of the explorer.
/// # Returns
/// The operation result.
pub(super) fn explorer_scroll(state: &TuiState, height: u16) -> u16 {
    let visible = height.saturating_sub(2).max(1);
    let cursor_line = state.cursor.saturating_add(1);
    cursor_line
        .saturating_sub(visible as usize - 1)
        .try_into()
        .unwrap_or(u16::MAX)
}
