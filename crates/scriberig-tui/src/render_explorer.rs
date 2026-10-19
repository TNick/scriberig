//! Renders the semantic explorer viewport and its selected row.

use crate::state::{Focus, TuiState};

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
    let mut rows = vec![display_row(
        root,
        &if state.cursor == 0 && state.focus == Focus::Explorer {
            "> ".to_owned()
        } else {
            "  ".to_owned()
        },
    )];
    rows.extend(state.explorer.rows.iter().enumerate().map(|(index, row)| {
        let cursor = index + 1;
        display_row(
            row,
            &if state.cursor == cursor && state.focus == Focus::Explorer {
                "> ".to_owned()
            } else {
                "  ".to_owned()
            },
        )
    }));
    if state.explorer.next.is_some() {
        rows.push("  … more entities; press Enter on the root".to_owned());
    }
    format!("Explorer\n{}", rows.join("\n"))
}

/// Formats one explorer row with its readable label and semantic kind.
///
/// - `row`: Explorer row to format.
/// - `prefix`: Selection marker and indentation prefix.
///
/// A compact human-readable explorer line.
/// # Returns
/// The operation result.
fn display_row(row: &crate::explorer::ExplorerRow, prefix: &str) -> String {
    let kind = row
        .kind
        .as_deref()
        .map(|value| format!(" [{value}]"))
        .unwrap_or_default();
    format!("{prefix}{}{kind}", row.label)
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
