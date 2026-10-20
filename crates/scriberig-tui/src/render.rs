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
#[path = "render_conversation.rs"]
mod render_conversation;
#[path = "render_explorer.rs"]
mod render_explorer;
#[path = "render_session.rs"]
mod render_session;
use render_explorer::{explorer_scroll, explorer_text};

/// Renders one complete shell frame without semantic I/O.
///
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
            .chain(crate::slash_commands::COMMANDS.iter().map(|command| {
                format!(
                    "{}{}: {}",
                    command.name, command.argument_hint, command.summary
                )
            }))
            .collect::<Vec<_>>()
            .join("\n"),
        crate::overlay::Overlay::SessionList(rows) => {
            if rows.is_empty() {
                "No saved conversations.".to_owned()
            } else {
                rows.join("\n")
            }
        }
        crate::overlay::Overlay::Palette(palette) => {
            let rows = palette
                .selector(area.height.saturating_sub(4).into())
                .render(area.width.into())
                .into_iter()
                .map(|line| line.as_str().to_owned())
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Search: {}\n{rows}",
                scriberig_tui_kit::ExternalText::new(&palette.query).as_str()
            )
        }
        crate::overlay::Overlay::Input(input) => format!(
            "{}: {}{}",
            match input.purpose {
                crate::overlay::input::InputPurpose::Entity => "Inspect entity",
                crate::overlay::input::InputPurpose::DiffSnapshot => "Diff target snapshot",
                crate::overlay::input::InputPurpose::FindQuery => "Find",
                crate::overlay::input::InputPurpose::ProjectionEdit => "Edit source",
            },
            input.editor.text().as_str(),
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
        Paragraph::new(render_helpers::safe_external_text(&text)).block(
            Block::default()
                .title(" Overlay [Esc close] ")
                .borders(Borders::ALL),
        ),
        area,
    );
}

/// Renders the context bar.
///
/// - `frame`: Terminal frame to draw.
/// - `area`: Context-bar rectangle.
/// - `state`: Cached presentation state.
fn render_context(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let project = state.project.as_deref().unwrap_or("no active project");
    let selection = state
        .session_snapshot
        .as_ref()
        .map(|session| &session.execution);
    let model = selection
        .and_then(|value| value.model_profile.as_deref().or(value.model.as_deref()))
        .unwrap_or("default model");
    let thinking = selection
        .and_then(|value| value.thinking_level.as_deref())
        .unwrap_or("default thinking");
    let text = format!(" scriberig · {project} · {model} · {thinking} ");
    frame.render_widget(
        Paragraph::new(render_helpers::safe_external_text(&text))
            .block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

/// Renders responsive body panes.
///
/// - `frame`: Terminal frame to draw.
/// - `layout`: Computed shell layout.
/// - `state`: Cached presentation state.
#[allow(clippy::too_many_lines)] // The responsive shell body shares one terminal layout pass.
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
            Paragraph::new(primary_view_text(
                state,
                regions[0].width,
                regions[0].height,
            ))
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
            Paragraph::new(render_helpers::safe_external_text(&content))
                .wrap(Wrap { trim: false })
                .block(panel_block(&title, state.session_composer_focused)),
            regions[1],
        );
    } else if state.primary_view == crate::selection::PrimaryView::Conversation {
        let regions =
            Layout::vertical([Constraint::Min(2), Constraint::Length(4)]).split(layout.primary);
        frame.render_widget(
            Paragraph::new(primary_view_text(
                state,
                regions[0].width,
                regions[0].height,
            ))
            .scroll(primary_scroll(state, regions[0].height))
            .block(panel_block(" Conversation ", true)),
            regions[0],
        );
        let composer = &state.conversation_composer;
        let editor_width = regions[1].width.saturating_sub(2).into();
        let editor_lines = if composer.editor.text().as_str() == composer.draft {
            composer.editor.render(editor_width)
        } else {
            let editor = scriberig_tui_kit::Composer::new();
            editor.replace_text(&composer.draft);
            editor.render(editor_width)
        };
        let target = match composer.target {
            crate::state::ConversationTarget::NewSession => "New conversation".to_owned(),
            crate::state::ConversationTarget::Existing {
                reply_anchor: scriberig_session::ContinuationAnchor::BranchHead { .. },
                ..
            } => "Conversation · replying at branch head".to_owned(),
            crate::state::ConversationTarget::Existing { .. } => {
                "Conversation · replying to earlier activity".to_owned()
            }
        };
        let status = if composer.pending {
            "Sending…"
        } else if state.conversation_browsing {
            "Esc edit · Enter pin reply target"
        } else {
            "Enter send · Ctrl+J newline · Esc browse"
        };
        let error = composer
            .error
            .as_ref()
            .map_or_else(Vec::new, |message| vec![message.clone()]);
        let completion = crate::slash_commands::suggestions(&composer.draft)
            .first()
            .map_or_else(String::new, |command| format!(" · {}", command.name));
        let content = std::iter::once(format!("{target} · {status}{completion}"))
            .chain(
                editor_lines
                    .into_iter()
                    .map(|line| line.as_str().to_owned()),
            )
            .chain(error)
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(render_helpers::safe_external_text(&content.join("\n")))
                .wrap(Wrap { trim: false })
                .block(panel_block(" Message ", true)),
            regions[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(primary_view_text(
                state,
                layout.primary.width,
                layout.primary.height,
            ))
            .scroll(primary_scroll(state, layout.primary.height))
            .block(panel_block(
                render_helpers::primary_view_title(state.primary_view),
                state.focus == Focus::Primary,
            )),
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
/// - `state`: Cached presentation state.
/// - `width`: Available primary view width.
/// - `height`: Available primary view height.
///
/// Styled session rows or plain text for other primary views.
/// # Returns
/// The operation result.
fn primary_view_text(state: &TuiState, width: u16, height: u16) -> Text<'static> {
    match state.primary_view {
        crate::selection::PrimaryView::Session => render_session::session_text(
            state,
            width.saturating_sub(2) as usize,
            height.saturating_sub(2) as usize,
        ),
        crate::selection::PrimaryView::Conversation => {
            render_conversation::conversation_text(state, width.saturating_sub(2) as usize)
        }
        crate::selection::PrimaryView::Explorer => {
            Text::from(render_explorer::explorer_text(state))
        }
        _ => Text::from(render_helpers::safe_external_text(&primary_text(state))),
    }
}

/// Converts external terminal content to visible safe text at the render edge.
///
/// - `text`: Content assembled from shared or user-provided values.
///
/// Plain text with terminal controls displayed as visible escapes.
/// Keeps the selected row visible in the active primary session list.
///
/// - `state`: Presentation state with the active row cursor.
/// - `height`: Primary panel height, including its borders.
///
/// Vertical and horizontal offsets for the primary text widget.
/// # Returns
/// The operation result.
fn primary_scroll(state: &TuiState, height: u16) -> (u16, u16) {
    if state.primary_view == crate::selection::PrimaryView::Explorer {
        return (explorer_scroll(state, height), 0);
    }
    if state.focus != Focus::Primary {
        return (0, 0);
    }
    if state.primary_view == crate::selection::PrimaryView::Conversation {
        return conversation_scroll(state, height.saturating_sub(2).max(1) as usize);
    }
    (0, 0)
}

/// Keeps the active semantic conversation location inside the visible tree.
///
/// - `state`: TUI state with its stable conversation cursor.
/// - `visible_rows`: Available conversation rows below the heading.
///
/// Vertical scroll offset for the current branch subtree.
/// # Returns
/// The operation result.
fn conversation_scroll(state: &TuiState, visible_rows: usize) -> (u16, u16) {
    let Some(model) = &state.conversation else {
        return (0, 0);
    };
    let Some(cursor) = state.conversation_cursor else {
        return (0, 0);
    };
    let Some(index) = model.index_of(cursor) else {
        return (0, 0);
    };
    let end = if let crate::view::conversation::ConversationSource::Branch(_) = cursor {
        let depth = model.rows[index].depth;
        model.rows[index + 1..]
            .iter()
            .take_while(|row| row.depth > depth)
            .count()
            + index
    } else {
        index
    };
    let first_visible = end.saturating_add(1).saturating_sub(visible_rows);
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
