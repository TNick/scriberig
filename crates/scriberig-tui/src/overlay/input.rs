//! Defines the presentation model for text input.

/// Text input state owned by an input overlay.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputPurpose {
    /// Parse the input as an entity identifier for inspection.
    #[default]
    Entity,
    /// Parse the input as the right-hand snapshot of a diff.
    DiffSnapshot,
    /// Search text for snapshot-scoped discovery.
    FindQuery,
    /// Enter a replacement expression for the selected source projection.
    ProjectionEdit,
}

/// Text input state owned by an input overlay.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InputState {
    /// Current input value.
    pub text: String,
    /// Reusable editor that owns cursor and safe text presentation behavior.
    pub editor: Box<scriberig_tui_kit::Composer>,
    /// Safe validation message retained while input remains active.
    pub error: Option<String>,
    /// Semantic operation that consumes the submitted text.
    pub purpose: InputPurpose,
}

impl InputState {
    /// Inserts input data, including pasted text, at the editor cursor.
    ///
    /// - `text`: Input text inserted at the editor cursor.
    pub fn append(&mut self, text: &str) {
        self.sync_editor();
        self.editor.insert_text(text);
        self.editor.text().as_str().clone_into(&mut self.text);
    }
    /// Removes the final Unicode scalar from the input.
    pub fn backspace(&mut self) {
        self.sync_editor();
        self.editor.delete_backward();
        self.editor.text().as_str().clone_into(&mut self.text);
    }

    /// Restores editor content after direct state initialization.
    fn sync_editor(&self) {
        if self.editor.text().as_str() != self.text {
            self.editor.replace_text(&self.text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InputState;

    /// Verifies the shared editor retains structure and safe control text.
    #[test]
    fn input_editor_handles_multiline_and_control_text() {
        let mut input = InputState::default();
        input.append("A\n界");
        assert_eq!(input.text, "A\n界");

        input.backspace();
        assert_eq!(input.text, "A\n");

        input.append("\u{1b}");
        assert_eq!(input.text, "A\n\\u{1b}");
    }
}
