//! Renders the deterministic presentation shell from state only.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    keymap,
    layout::{LayoutClass, ShellLayout, shell_layout},
    state::{Focus, TuiState},
};

#[path = "render_helpers.rs"]
mod render_helpers;
use render_helpers::{inspector_text, panel_block, render_status};
#[path = "render_primary.rs"]
mod render_primary;
pub(crate) use render_primary::primary_text;
#[path = "render_explorer.rs"]
mod render_explorer;
#[path = "render_session.rs"]
mod render_session;
use render_explorer::{explorer_scroll, explorer_text};

/// Renders one complete shell frame without semantic I/O.
///
/// # Parameters
/// - `frame`: Terminal frame to draw.
/// - `state`: Cached presentation state.
pub fn render(frame: &mut Frame<'_>, state: &TuiState) {
    let layout = shell_layout(frame.area());
    render_context(frame, layout.context, state);
    render_body(frame, &layout, state);
    render_overlay(frame, layout.primary, state);
    render_status(frame, layout.status, state);
}

/// Renders the active modal overlay as a visible, text-labelled panel.
///
/// # Parameters
/// - `frame`: Terminal frame to draw.
/// - `area`: Overlay rectangle.
/// - `state`: Cached presentation state.
fn render_overlay(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let Some(overlay) = state.overlay.as_ref() else {
        return;
    };
    frame.render_widget(Clear, area);
    let text = match overlay {
        crate::overlay::Overlay::Help => crate::overlay::help::rows()
            .into_iter()
            .map(|row| format!("{}: {}", row.key, row.action))
            .collect::<Vec<_>>()
            .join("\n"),
        crate::overlay::Overlay::Palette(palette) => {
            let rows = palette
                .filtered()
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    let marker = if index == palette.selected {
                        "> "
                    } else {
                        "  "
                    };
                    format!("{marker}{}", item.label)
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("Search: {}\n{rows}", palette.query)
        }
        crate::overlay::Overlay::Input(input) => format!(
            "{}: {}{}",
            match input.purpose {
                crate::overlay::input::InputPurpose::Entity => "Inspect entity",
                crate::overlay::input::InputPurpose::DiffSnapshot => "Diff target snapshot",
                crate::overlay::input::InputPurpose::FindQuery => "Find",
                crate::overlay::input::InputPurpose::ProjectionEdit => "Edit source",
            },
            input.text,
            input
                .error
                .as_deref()
                .map_or(String::new(), |error| format!("\nError: {error}")),
        ),
        crate::overlay::Overlay::Confirm(confirm) => format!(
            "Action: {}\nTarget: {}\nBase: {}\n{}\n\nEnter confirm | Esc cancel",
            confirm.action,
            crate::inspector::sanitize(&confirm.target),
            crate::inspector::sanitize(&confirm.base),
            crate::inspector::sanitize(&confirm.summary),
        ),
        crate::overlay::Overlay::Composer(composer) => format!(
            "Session: {}\nBranch: {}\n\n{}\n\nEnter submit | Esc close",
            composer.session,
            composer
                .branch
                .map_or_else(|| "default".to_owned(), |value| value.to_string()),
            crate::inspector::sanitize(&composer.text),
        ),
    };
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title(" Overlay [Esc close] ")
                .borders(Borders::ALL),
        ),
        area,
    );
}

/// Renders the context bar.
///
/// # Parameters
/// - `frame`: Terminal frame to draw.
/// - `area`: Context-bar rectangle.
/// - `state`: Cached presentation state.
fn render_context(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let overlay = if state.overlay.is_some() {
        " | OVERLAY"
    } else {
        ""
    };
    let connection = match state.connection {
        crate::state::ConnectionState::Connecting => "connecting",
        crate::state::ConnectionState::Connected => "connected",
        crate::state::ConnectionState::Disconnected => "disconnected",
        crate::state::ConnectionState::Failed => "failed",
    };
    let stale_marker = if state.stale { " | STALE" } else { "" };
    let text = format!(
        " Scriberig | service: {connection}{stale_marker} | project: {} | focus: {:?}{overlay} ",
        state.project.as_deref().unwrap_or("unselected"),
        state.focus
    );
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

/// Renders responsive body panes.
///
/// # Parameters
/// - `frame`: Terminal frame to draw.
/// - `layout`: Computed shell layout.
/// - `state`: Cached presentation state.
fn render_body(frame: &mut Frame<'_>, layout: &ShellLayout, state: &TuiState) {
    if layout.class == LayoutClass::TooSmall {
        frame.render_widget(
            Paragraph::new("Terminal too small. Resize to at least 20x8. Press q to quit.")
                .block(Block::default().borders(Borders::ALL)),
            layout.primary,
        );
        return;
    }
    if let Some(area) = layout.explorer {
        frame.render_widget(
            Paragraph::new(explorer_text(state))
                .scroll((explorer_scroll(state, area.height), 0))
                .block(panel_block(" Explorer ", state.focus == Focus::Explorer)),
            area,
        );
    }
    if state.primary_view == crate::selection::PrimaryView::Session
        && let Some(composer) = state.session_composer.as_ref()
    {
        let regions =
            Layout::vertical([Constraint::Min(2), Constraint::Length(5)]).split(layout.primary);
        frame.render_widget(
            Paragraph::new(primary_view_text(state, regions[0].width))
                .scroll(primary_scroll(state, regions[0].height))
                .block(panel_block(" Session ", state.focus == Focus::Primary)),
            regions[0],
        );
        let title = if state.session_composer_focused {
            format!(
                " Agent composer · {} · Enter send · Ctrl+J newline · Esc browse ",
                composer.session
            )
        } else {
            format!(" Agent composer · {} · press i to edit ", composer.session)
        };
        let draft = if composer.text.is_empty() {
            "Write a message..."
        } else {
            composer.text.as_str()
        };
        let content = composer
            .error
            .as_ref()
            .map_or_else(|| draft.to_owned(), |error| format!("{draft}\n{error}"));
        frame.render_widget(
            Paragraph::new(content)
                .wrap(Wrap { trim: false })
                .block(panel_block(&title, state.session_composer_focused)),
            regions[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(primary_view_text(state, layout.primary.width))
                .scroll(primary_scroll(state, layout.primary.height))
                .block(panel_block(" Primary ", state.focus == Focus::Primary)),
            layout.primary,
        );
    }
    if let Some(area) = layout.inspector {
        frame.render_widget(
            Paragraph::new(inspector_text(state))
                .block(panel_block(" Inspector ", state.focus == Focus::Inspector)),
            area,
        );
    }
}

/// Builds the active primary view with depth-aware session rail styles.
///
/// # Parameters
/// - `state`: Cached presentation state.
/// - `width`: Available primary view width.
///
/// # Returns
/// Styled session rows or plain text for other primary views.
fn primary_view_text(state: &TuiState, width: u16) -> Text<'static> {
    if state.primary_view == crate::selection::PrimaryView::Session {
        render_session::session_text(state, width.saturating_sub(2) as usize)
    } else {
        Text::from(primary_text(state))
    }
}

/// Keeps the selected row visible in the active primary session list.
///
/// # Parameters
/// - `state`: Presentation state with the active row cursor.
/// - `height`: Primary panel height, including its borders.
///
/// # Returns
/// Vertical and horizontal offsets for the primary text widget.
fn primary_scroll(state: &TuiState, height: u16) -> (u16, u16) {
    if state.focus != Focus::Primary || state.primary_view != crate::selection::PrimaryView::Session
    {
        return (0, 0);
    }
    let visible_rows = height.saturating_sub(2).max(1) as usize;
    // The session heading occupies the first paragraph row before the cursor-indexed rows.
    let first_visible = state.cursor.saturating_add(2).saturating_sub(visible_rows);
    (u16::try_from(first_visible).unwrap_or(u16::MAX), 0)
}

#[cfg(test)]
#[path = "render_context_tests.rs"]
mod context_tests;
#[cfg(test)]
#[path = "render_narrow_tests.rs"]
mod narrow_tests;
#[cfg(test)]
#[path = "render_style_tests.rs"]
mod style_tests;
#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
