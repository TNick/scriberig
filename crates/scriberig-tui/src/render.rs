//! Renders the deterministic presentation shell from state only.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    layout::{LayoutClass, ShellLayout, shell_layout},
    state::TuiState,
};

/// Renders one complete shell frame without semantic I/O.
pub fn render(frame: &mut Frame<'_>, state: &TuiState) {
    let layout = shell_layout(frame.area());
    render_context(frame, layout.context, state);
    render_body(frame, &layout, state);
    render_status(frame, layout.status, state);
    render_hints(frame, layout.hints);
}

/// Renders the context bar.
fn render_context(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let text = format!(" Scriberig | local shell | focus: {:?} ", state.focus);
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
            Paragraph::new("Explorer\n  No semantic data loaded")
                .block(Block::default().title(" Explorer ").borders(Borders::ALL)),
            area,
        );
    }
    frame.render_widget(
        Paragraph::new("Shell ready\n\nSemantic operations are not attached in this runtime.")
            .block(Block::default().title(" Primary ").borders(Borders::ALL)),
        layout.primary,
    );
    if let Some(area) = layout.inspector {
        frame.render_widget(
            Paragraph::new("No selection")
                .block(Block::default().title(" Inspector ").borders(Borders::ALL)),
            area,
        );
    }
    let _ = state;
}

/// Renders the runtime status line.
fn render_status(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    frame.render_widget(Paragraph::new(state.shell_message.as_str()), area);
}

/// Renders keyboard hints shared by all responsive layouts.
fn render_hints(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(Paragraph::new(" q/Esc quit | ? help | resize-safe "), area);
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
