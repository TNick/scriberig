//! Renders the semantic explorer viewport and its selected row.

use crate::state::{Focus, TuiState};

/// Formats the bounded semantic explorer model.
///
/// # Parameters
/// - `state`: Cached presentation state.
/// - `super`: Parent module visibility marker.
///
/// # Returns
/// Text representing the explorer pane.
pub(super) fn explorer_text(state: &TuiState) -> String {
    let Some(root) = state.explorer.root.as_ref() else {
        return "Explorer\n  No semantic root entered".to_owned();
    };
    let mut rows = vec![format!(
        "{}{}",
        if state.cursor == 0 && state.focus == Focus::Explorer {
            "> "
        } else {
            "  "
        },
        root.label
    )];
    rows.extend(state.explorer.rows.iter().enumerate().map(|(index, row)| {
        let cursor = index + 1;
        format!(
            "{}{}",
            if state.cursor == cursor && state.focus == Focus::Explorer {
                "> "
            } else {
                "  "
            },
            row.label
        )
    }));
    if state.explorer.next.is_some() {
        rows.push("  … more entities; press Enter on the root".to_owned());
    }
    format!("Explorer\n{}", rows.join("\n"))
}

/// Computes the explorer text offset needed to keep the cursor in view.
///
/// # Parameters
/// - `state`: Current TUI presentation state.
/// - `height`: Outer explorer pane height.
/// - `super`: Parent module visibility marker.
///
/// # Returns
/// The number of text rows to skip from the top of the explorer.
pub(super) fn explorer_scroll(state: &TuiState, height: u16) -> u16 {
    let visible = height.saturating_sub(2).max(1);
    let cursor_line = state.cursor.saturating_add(1);
    cursor_line
        .saturating_sub(visible as usize - 1)
        .try_into()
        .unwrap_or(u16::MAX)
}
