//! Wraps the upstream editor with a Scriberig-owned keyboard facade.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rpi_tui::{Component, Editor, EditorOptions, EditorStyle, Focusable, Keybindings};

use crate::text::ExternalText;

/// Describes cursor movement accepted by the generic composer.
///
/// # Public members
/// - `Left`: Moves one character left.
/// - `Right`: Moves one character right.
/// - `Start`: Moves to the start of the current line.
/// - `End`: Moves to the end of the current line.
/// - `Up`: Moves one line up.
/// - `Down`: Moves one line down.
/// - `DeleteForward`: Deletes the next character.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CursorMotion {
    /// Moves one character left.
    Left,
    /// Moves one character right.
    Right,
    /// Moves to the start of the current line.
    Start,
    /// Moves to the end of the current line.
    End,
    /// Moves one line up while retaining the preferred column.
    Up,
    /// Moves one line down while retaining the preferred column.
    Down,
    /// Deletes the character after the cursor.
    DeleteForward,
}

/// Provides editor behavior while keeping upstream editor types private.
///
/// # Public members
/// - [`Composer::new`]: Creates an empty editor.
/// - [`Composer::replace_text`]: Replaces the draft with sanitized text.
/// - [`Composer::insert_text`]: Inserts sanitized text at the cursor.
/// - [`Composer::paste_text`]: Inserts a sanitized paste payload.
/// - [`Composer::delete_backward`]: Deletes the previous character.
/// - [`Composer::delete_forward`]: Deletes the next character.
/// - [`Composer::move_cursor`]: Moves the cursor.
/// - [`Composer::text`]: Reads the current safe draft.
/// - [`Composer::render`]: Renders safe plain lines.
/// - [`Composer::clear`]: Clears the current draft.
///
/// # Private members
/// - `editor`: Holds the upstream editor component.
pub struct Composer {
    editor: Editor,
}

impl Clone for Composer {
    /// Clones the visible draft into an independent upstream editor.
    ///
    /// # Returns
    /// An editor containing the same text, positioned at its end.
    fn clone(&self) -> Self {
        let copy = Self::new();
        let text = self.text();
        copy.replace_text(text.as_str());
        copy
    }
}

impl std::fmt::Debug for Composer {
    /// Formats the composer without exposing upstream implementation state.
    ///
    /// # Parameters
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// The formatter result.
    ///
    /// # Errors
    /// Returns formatting errors from the destination.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Composer")
            .field("text", &self.text().as_str())
            .finish()
    }
}

impl PartialEq for Composer {
    /// Compares composer content while ignoring disposable cursor position.
    ///
    /// # Parameters
    /// - `other`: Composer to compare.
    ///
    /// # Returns
    /// Whether the visible draft text matches.
    fn eq(&self, other: &Self) -> bool {
        self.text() == other.text()
    }
}

impl Eq for Composer {}

impl Composer {
    /// Creates an empty composer.
    ///
    /// # Returns
    /// A focused composer ready to receive input.
    #[must_use]
    pub fn new() -> Self {
        let editor = Editor::new(
            EditorOptions::default(),
            EditorStyle::default(),
            Arc::new(Keybindings::new()),
        );
        editor.set_focused(true);
        Self { editor }
    }

    /// Replaces the complete draft with sanitized text.
    ///
    /// # Parameters
    /// - `text`: External draft text.
    pub fn replace_text(&self, text: &str) {
        let safe_text = ExternalText::new(text);
        self.editor.set_text(safe_text.as_str());
        let lines = safe_text.as_str().split('\n').collect::<Vec<_>>();
        let last_row = lines.len().saturating_sub(1);
        let last_column = lines.last().map_or(0, |line| line.len());
        self.editor.set_cursor(last_row, last_column);
    }

    /// Inserts text at the current cursor position after sanitization.
    ///
    /// # Parameters
    /// - `text`: External text to insert.
    pub fn insert_text(&self, text: &str) {
        let safe_text = ExternalText::new(text);
        for character in safe_text.as_str().chars() {
            if character == '\n' {
                self.editor
                    .handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
            } else {
                self.editor
                    .handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
            }
        }
    }

    /// Inserts pasted text through the upstream paste path.
    ///
    /// # Parameters
    /// - `text`: External clipboard or terminal paste data.
    pub fn paste_text(&self, text: &str) {
        let safe_text = ExternalText::new(text);
        self.editor.handle_paste(safe_text.as_str());
    }

    /// Deletes the character immediately before the cursor.
    pub fn delete_backward(&self) {
        self.editor
            .handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    }

    /// Deletes the character immediately after the cursor.
    pub fn delete_forward(&self) {
        self.editor
            .handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    }

    /// Moves the cursor using the upstream editor's Unicode-aware behavior.
    ///
    /// # Parameters
    /// - `motion`: Direction or boundary to move toward.
    pub fn move_cursor(&self, motion: CursorMotion) {
        let code = match motion {
            CursorMotion::Left => KeyCode::Left,
            CursorMotion::Right => KeyCode::Right,
            CursorMotion::Start => KeyCode::Home,
            CursorMotion::End => KeyCode::End,
            CursorMotion::Up => KeyCode::Up,
            CursorMotion::Down => KeyCode::Down,
            CursorMotion::DeleteForward => KeyCode::Delete,
        };
        self.editor
            .handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    /// Returns the current draft as safe plain text.
    ///
    /// # Returns
    /// The sanitized composer content.
    pub fn text(&self) -> ExternalText {
        ExternalText::new(&self.editor.get_expanded_text())
    }

    /// Renders the composer without returning upstream ANSI styling.
    ///
    /// # Parameters
    /// - `width`: Maximum rendering width in terminal columns.
    ///
    /// # Returns
    /// Safe plain text lines.
    pub fn render(&self, width: usize) -> Vec<ExternalText> {
        let rendered = self.editor.render(width);
        let content = rendered
            .iter()
            .skip(1)
            .take(rendered.len().saturating_sub(2));
        content
            .map(|line| {
                let visible_cursor = line.replace(rpi_tui::CURSOR_MARKER, "▏");
                ExternalText::new(&rpi_tui::strip_ansi(&visible_cursor))
            })
            .collect()
    }

    /// Clears text and returns the cursor to the start of an empty draft.
    pub fn clear(&self) {
        self.editor.clear();
    }
}

impl Default for Composer {
    /// Creates a default empty composer.
    ///
    /// # Returns
    /// A focused empty composer.
    fn default() -> Self {
        Self::new()
    }
}
