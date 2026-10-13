//! Renders the deterministic presentation shell from state only.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    keymap,
    layout::{LayoutClass, ShellLayout, shell_layout},
    state::{Focus, TuiState},
};

#[path = "render_helpers.rs"]
mod render_helpers;
use render_helpers::{inspector_text, panel_block, render_status};
#[path = "render_explorer.rs"]
mod render_explorer;
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
    frame.render_widget(
        Paragraph::new(primary_text(state))
            .block(panel_block(" Primary ", state.focus == Focus::Primary)),
        layout.primary,
    );
    if let Some(area) = layout.inspector {
        frame.render_widget(
            Paragraph::new(inspector_text(state))
                .block(panel_block(" Inspector ", state.focus == Focus::Inspector)),
            area,
        );
    }
}

/// Renders the selected primary view from cached presentation state.
///
/// # Parameters
/// - `state`: Cached presentation state.
///
/// # Returns
/// Text representing the selected primary view.
#[allow(clippy::too_many_lines)]
fn primary_text(state: &TuiState) -> String {
    match state.primary_view {
        crate::selection::PrimaryView::Overview => inspector_text(state),
        crate::selection::PrimaryView::History => match &state.history {
            crate::inspector::LoadState::Ready(model) => format!(
                "History @ {}\n{}",
                model.snapshot,
                model
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| {
                        format!(
                            "{}{}: {} -> {} ({} changed)",
                            if state.focus == Focus::Primary && state.cursor == index {
                                "> "
                            } else {
                                "  "
                            },
                            row.id,
                            row.base,
                            row.result,
                            row.entities.len()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            crate::inspector::LoadState::Loading { .. } => "Loading history...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "History not loaded".to_owned(),
        },
        crate::selection::PrimaryView::Find => match &state.find {
            crate::inspector::LoadState::Ready(model) => format!(
                "Find: {}\n{}",
                model.query,
                model
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| format!(
                        "{}{} {}{}",
                        if state.focus == Focus::Primary && state.cursor == index {
                            "> "
                        } else {
                            "  "
                        },
                        row.entity,
                        row.entity_type.as_deref().unwrap_or("unknown"),
                        row.preview
                            .as_deref()
                            .map_or(String::new(), |value| format!(" — {value}"))
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            crate::inspector::LoadState::Loading { .. } => "Searching...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "Find not run".to_owned(),
        },
        crate::selection::PrimaryView::Diff => match &state.diff {
            crate::inspector::LoadState::Ready(model) => format!(
                "Diff\nleft/base: {}\nright/target: {}\n{}",
                model.left,
                model.right,
                model
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| {
                        format!(
                            "{}{}: {:?} -> {:?}",
                            if state.focus == Focus::Primary && state.cursor == index {
                                "> "
                            } else {
                                "  "
                            },
                            row.entity,
                            row.before,
                            row.after
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            crate::inspector::LoadState::Loading { .. } => "Loading diff...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "Diff not loaded".to_owned(),
        },
        crate::selection::PrimaryView::Projection => match &state.projection {
            crate::inspector::LoadState::Ready(model) => format!(
                "Projection {}\nsource: {}\nstyle: {}\nbase: {}\n{}",
                model.projection.as_deref().unwrap_or("unknown"),
                model.source_entity,
                model.style,
                model.base_snapshot,
                model.content
            ),
            crate::inspector::LoadState::Loading { .. } => "Rendering projection...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "Projection not loaded".to_owned(),
        },
        crate::selection::PrimaryView::Changes => match &state.review {
            crate::inspector::LoadState::Ready(model) => {
                let fields = model.change.splitn(3, '|').collect::<Vec<_>>();
                let selected = model.conflicts.get(state.cursor);
                let review_diff = selected.and_then(|value| {
                    let (scope_and_path, rest) = value.split_once("add | ")?;
                    let prefix = scope_and_path
                        .trim_end_matches(": ")
                        .replace("; ", "\npath: ");
                    Some(format!("{prefix}\n{}", rest.replace(" | ", "\n")))
                });
                let detail = if let Some(diff) = review_diff {
                    diff
                } else if fields.len() == 3 && fields[0] == "project-edit" {
                    format!("path: src/lib.rs\nold: left + right\nnew: {}", fields[2])
                } else if let Some(change) = selected {
                    format!(
                        "candidate: {}\nfile {}/{}\n{}",
                        model.change,
                        state.cursor + 1,
                        model.conflicts.len(),
                        change
                    )
                } else {
                    format!("change: {}", model.change)
                };
                format!(
                    "Change review\nbase: {}\nvalidation: {:?}\n{}\nUp/Down select file | Enter to confirm",
                    model.base_snapshot, model.validation, detail,
                )
            }
            crate::inspector::LoadState::Loading { .. } => "Preparing change review...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "No change review".to_owned(),
        },
        crate::selection::PrimaryView::Session => match &state.session {
            crate::inspector::LoadState::Ready(model) => model
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    format!(
                        "{}{}{}: {}",
                        if state.focus == Focus::Primary && state.cursor == index {
                            "> "
                        } else {
                            "  "
                        },
                        "  ".repeat(row.depth),
                        if row.transient { "~" } else { "" },
                        row.label
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
                .chain(state.agent_output_observations.iter().map(|output| {
                    format!(
                        "[stream, non-canonical] {}",
                        crate::inspector::sanitize(output)
                    )
                }))
                .collect::<Vec<_>>()
                .join("\n"),
            crate::inspector::LoadState::Loading { .. } => "Loading session...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "Session not loaded".to_owned(),
        },
        crate::selection::PrimaryView::Operations => {
            if state.operations.is_empty() {
                "No observed operations".to_owned()
            } else {
                state
                    .operations
                    .iter()
                    .enumerate()
                    .map(|(index, operation)| {
                        format!(
                            "{}{} [{:?}] {}",
                            if state.focus == Focus::Primary && state.cursor == index {
                                "> "
                            } else {
                                "  "
                            },
                            operation.id,
                            operation.status,
                            operation.kind
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
