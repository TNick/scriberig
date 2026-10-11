//! Defines the presentation model for text input.

/// Text input state owned by an input overlay.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InputState {
    /// Current input value.
    pub text: String,
    /// Safe validation message retained while input remains active.
    pub error: Option<String>,
}

impl InputState {
    /// Appends input data, including pasted text, without interpreting it.
    pub fn append(&mut self, text: &str) {
        self.text.push_str(text);
    }
    /// Removes the final Unicode scalar from the input.
    pub fn backspace(&mut self) {
        self.text.pop();
    }
}
