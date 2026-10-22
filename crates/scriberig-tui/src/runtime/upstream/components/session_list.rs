//! Renders the saved-session list overlay and routes its close/select keys.

use std::any::Any;
use std::sync::mpsc::Sender;

use rpi_tui::{Component, InputEvent, KeyCode};

use super::super::ComposerInput;

/// Modal list of saved conversations with stable IDs for follow-up commands.
///
/// # Public members
/// None.
///
/// # Private members
/// - `rows`: Preformatted session rows, one per saved conversation.
/// - `input_sender`: Channel used to close the overlay or select a session.
pub(crate) struct SessionListPanel {
    /// Preformatted session rows.
    rows: Vec<String>,
    /// Channel used to close the overlay or select a session.
    input_sender: Sender<ComposerInput>,
}

impl SessionListPanel {
    /// Creates a session-list panel from reducer-owned rows.
    ///
    /// # Parameters
    /// - `rows`: Formatted session rows.
    /// - `input_sender`: Channel for close and selection requests.
    ///
    /// # Returns
    /// A panel that renders and handles the saved-session list.
    pub(crate) fn new(rows: Vec<String>, input_sender: Sender<ComposerInput>) -> Self {
        Self { rows, input_sender }
    }
}

impl Component for SessionListPanel {
    /// Renders the session list with a title and close hint.
    ///
    /// # Parameters
    /// - `width`: Available terminal width.
    ///
    /// # Returns
    /// Rendered list lines.
    fn render(&self, width: usize) -> Vec<String> {
        let inner = width.saturating_sub(2).max(8);
        let mut lines = vec![truncate("Saved conversations", inner)];
        lines.push(truncate("Enter switch · Esc close", inner));
        lines.push(String::new());
        for row in &self.rows {
            lines.push(truncate(row, inner));
        }
        lines
    }

    /// Closes on Escape or starts a switch for the first row on Enter.
    ///
    /// # Parameters
    /// - `event`: Normalized terminal event.
    ///
    /// # Returns
    /// `true` when the event was consumed.
    fn handle_event(&self, event: &InputEvent) -> bool {
        let InputEvent::Key(key) = event else {
            return false;
        };
        match key.code {
            KeyCode::Esc => {
                let _ = self.input_sender.send(ComposerInput::CancelSelection);
                true
            }
            KeyCode::Enter => {
                let Some(session) = self
                    .rows
                    .first()
                    .and_then(|row| row.split_whitespace().next())
                else {
                    return false;
                };
                let _ = self
                    .input_sender
                    .send(ComposerInput::Submit(format!("/switch {session}")));
                true
            }
            _ => false,
        }
    }

    /// Invalidates no cached frame because rows are rendered directly.
    fn invalidate(&self) {}

    /// Supports upstream layout-node inspection.
    ///
    /// # Returns
    /// A type-erased reference to this component.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Truncates one row to the available display width.
///
/// # Parameters
/// - `value`: Row text.
/// - `width`: Maximum character count.
///
/// # Returns
/// The truncated row.
fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}
