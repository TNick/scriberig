//! Provides focused helpers for the TUI shell renderer.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Builds a panel border with a visible focus state.
///
/// - `title`: Panel title displayed in the border.
/// - `focused`: Whether this panel currently owns keyboard focus.
/// - `super`: Parent-module visibility marker.
///
/// A styled panel block.
/// # Returns
/// The operation result.
pub(super) fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let title = if focused {
        format!("{title}[ACTIVE]")
    } else {
        title.to_owned()
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
}

/// Formats inspector loading, error, and ready states with exact identifiers.
///
/// - `state`: Cached presentation state.
/// - `super`: Parent-module visibility marker.
///
/// Text representing inspector state.
/// # Returns
/// The operation result.
pub(super) fn inspector_text(state: &TuiState) -> String {
    match &state.inspector {
        crate::inspector::LoadState::NotRequested => {
            "Shell ready\n\nEnter an entity with : or /.".to_owned()
        }
        crate::inspector::LoadState::Loading { request_id } => {
            format!("Inspecting entity (request {request_id})...")
        }
        crate::inspector::LoadState::Failed {
            category,
            message,
            stale,
        } => format!(
            "Error [{category}]{}\n{message}",
            if *stale { " (stale data retained)" } else { "" }
        ),
        crate::inspector::LoadState::Ready(model) => format!(
            "Entity: {}\nRevision: {}\nSnapshot: {}\nKind: {}\nLabel: {}\n{}",
            model.entity,
            model
                .revision
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            model
                .snapshot
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            model.kind.as_deref().unwrap_or("unknown"),
            model.label.as_deref().unwrap_or("none"),
            model.description.as_deref().unwrap_or("No description"),
        ),
    }
}

/// Renders the runtime status line.
///
/// - `frame`: Terminal frame to draw.
/// - `area`: Status-line rectangle.
/// - `state`: Cached presentation state.
/// - `super`: Parent-module visibility marker.
pub(super) fn render_status(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let hints = keymap::hints(state)
        .into_iter()
        .map(|(key, label)| format!("{key} {label}"))
        .collect::<Vec<_>>()
        .join(" | ");
    frame.render_widget(Paragraph::new(format!(" {hints} ")), area);
}
