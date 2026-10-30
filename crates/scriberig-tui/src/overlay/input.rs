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
    /// Safe validation message retained while input remains active.
    pub error: Option<String>,
    /// Semantic operation that consumes the submitted text.
    pub purpose: InputPurpose,
}

impl InputState {
    /// Appends input data after making terminal controls visible.
    ///
    /// - `text`: Input text inserted at the editor cursor.
    pub fn append(&mut self, text: &str) {
        self.text
            .push_str(&crate::inspector::sanitize_external_text(text));
    }
    /// Removes the final Unicode scalar from the input.
    pub fn backspace(&mut self) {
        self.text.pop();
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
