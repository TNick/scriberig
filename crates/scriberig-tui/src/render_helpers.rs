//! Provides focused helpers for the TUI shell renderer.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Returns the stable title for the selected auxiliary primary view.
///
/// - `view`: Current semantic product surface.
///
/// A user-facing view name for the primary panel.
/// # Returns
/// The selected view's title.
pub(super) fn primary_view_title(view: crate::selection::PrimaryView) -> &'static str {
    match view {
        crate::selection::PrimaryView::Conversation => "Conversation",
        crate::selection::PrimaryView::Overview => "Overview",
        crate::selection::PrimaryView::Explorer => "Explorer",
        crate::selection::PrimaryView::History => "History",
        crate::selection::PrimaryView::Find => "Find",
        crate::selection::PrimaryView::Diff => "Diff",
        crate::selection::PrimaryView::Projection => "Projection",
        crate::selection::PrimaryView::Changes => "Changes",
        crate::selection::PrimaryView::Session => "Session",
        crate::selection::PrimaryView::Operations => "Operations",
    }
}

/// Converts external terminal content to visible safe text at the render edge.
///
/// - `text`: Content assembled from shared or user-provided values.
///
/// Plain text with terminal controls displayed as visible escapes.
/// # Returns
/// The sanitized text.
pub(super) fn safe_external_text(text: &str) -> String {
    scriberig_tui_kit::ExternalText::new(text)
        .as_str()
        .to_owned()
}

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

/// Resolves one branch-depth role through the shared TUI theme facade.
///
/// - `depth`: Zero-based branch ancestry depth.
///
/// A finite palette color that cycles for deeper branches.
/// # Returns
/// The Ratatui color for the ancestry level.
pub(super) fn depth_color(depth: usize) -> Color {
    match scriberig_tui_kit::style_for(scriberig_tui_kit::Tone::BranchDepth(depth)).foreground {
        scriberig_tui_kit::PaletteColor::Default => Color::Reset,
        scriberig_tui_kit::PaletteColor::Ansi256(index) => Color::Indexed(index),
    }
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
