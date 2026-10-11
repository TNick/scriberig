//! Renders the deterministic presentation shell from state only.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    keymap,
    layout::{LayoutClass, ShellLayout, shell_layout},
    state::TuiState,
};

/// Renders one complete shell frame without semantic I/O.
pub fn render(frame: &mut Frame<'_>, state: &TuiState) {
    let layout = shell_layout(frame.area());
    render_context(frame, layout.context, state);
    render_body(frame, &layout, state);
    render_overlay(frame, layout.primary, state);
    render_status(frame, layout.status, state);
    render_hints(frame, layout.hints, state);
}

/// Renders the active modal overlay as a visible, text-labelled panel.
fn render_overlay(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let Some(overlay) = state.overlay.as_ref() else {
        return;
    };
    let text = match overlay {
        crate::overlay::Overlay::Help => crate::overlay::help::rows()
            .into_iter()
            .map(|row| format!("{}: {}", row.key, row.action))
            .collect::<Vec<_>>()
            .join("\n"),
        crate::overlay::Overlay::Palette(palette) => format!(
            "Search: {}\n{}",
            palette.query,
            palette
                .filtered()
                .into_iter()
                .map(|item| item.label)
                .collect::<Vec<_>>()
                .join("\n")
        ),
        crate::overlay::Overlay::Input(input) => format!(
            "Find: {}{}",
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
fn render_context(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let overlay = if state.overlay.is_some() {
        " | OVERLAY"
    } else {
        ""
    };
    let text = format!(
        " Scriberig | local shell | focus: {:?}{} ",
        state.focus, overlay
    );
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

/// Renders responsive body panes.
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
                .block(Block::default().title(" Explorer ").borders(Borders::ALL)),
            area,
        );
    }
    frame.render_widget(
        Paragraph::new(primary_text(state))
            .block(Block::default().title(" Primary ").borders(Borders::ALL)),
        layout.primary,
    );
    if let Some(area) = layout.inspector {
        frame.render_widget(
            Paragraph::new(primary_text(state))
                .block(Block::default().title(" Inspector ").borders(Borders::ALL)),
            area,
        );
    }
}

/// Formats the bounded semantic explorer model.
fn explorer_text(state: &TuiState) -> String {
    state.explorer.root.as_ref().map_or_else(
        || "Explorer\n  No semantic root entered".to_owned(),
        |row| format!("Explorer\n  > {}", row.label),
    )
}

/// Renders the selected primary view from cached presentation state.
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
                    .map(|row| format!("{}: {} -> {}", row.id, row.base, row.result))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            crate::inspector::LoadState::Loading { .. } => "Loading history...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "History not loaded".to_owned(),
        },
        crate::selection::PrimaryView::Diff => match &state.diff {
            crate::inspector::LoadState::Ready(model) => format!(
                "Diff\nleft/base: {}\nright/target: {}\n{}",
                model.left,
                model.right,
                model
                    .rows
                    .iter()
                    .map(|row| format!("{}: {:?} -> {:?}", row.entity, row.before, row.after))
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
            crate::inspector::LoadState::Ready(model) => format!(
                "Change review: {}\nbase: {}\naffected: {}\nvalidation: {:?}\nEnter to confirm",
                model.change,
                model.base_snapshot,
                model.affected_entities.len(),
                model.validation
            ),
            crate::inspector::LoadState::Loading { .. } => "Preparing change review...".to_owned(),
            crate::inspector::LoadState::Failed { message, .. } => message.clone(),
            crate::inspector::LoadState::NotRequested => "No change review".to_owned(),
        },
    }
}

/// Formats inspector loading, error, and ready states with exact identifiers.
fn inspector_text(state: &TuiState) -> String {
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
fn render_status(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    frame.render_widget(Paragraph::new(state.shell_message.as_str()), area);
}

/// Renders keyboard hints shared by all responsive layouts.
fn render_hints(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let hints = keymap::hints(state)
        .into_iter()
        .map(|(key, label)| format!("{key} {label}"))
        .collect::<Vec<_>>()
        .join(" | ");
    frame.render_widget(Paragraph::new(format!(" {hints} ")), area);
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::render;
    use crate::state::TuiState;

    /// Renders representative sizes without touching a real terminal.
    #[test]
    fn renders_all_required_sizes() {
        for (width, height) in [(140, 40), (100, 30), (72, 24), (10, 3)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("terminal");
            terminal
                .draw(|frame| render(frame, &TuiState::default()))
                .expect("draw");
        }
    }

    /// Verifies the narrow shell preserves a readable primary message.
    #[test]
    fn narrow_frame_contains_primary_text() {
        let backend = TestBackend::new(72, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame, &TuiState::default()))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let rendered: String = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(rendered.contains("Shell ready"));
    }
}
