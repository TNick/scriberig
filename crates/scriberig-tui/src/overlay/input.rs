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
    /// Appends input data, including pasted text, without interpreting it.
    /// - `text`: Argument named `text`.
    pub fn append(&mut self, text: &str) {
        self.text.push_str(text);
    }
    /// Removes the final Unicode scalar from the input.
    pub fn backspace(&mut self) {
        self.text.pop();
    }
}
