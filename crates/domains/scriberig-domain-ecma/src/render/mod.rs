//! Renders the semantic source model to text under one exact style.
//!
//! Rendering is deterministic: the same model and style always produce the same
//! bytes, and every emitted construct is inside the subset described in
//! [`crate::source`], so the output re-parses to the same model.

mod expressions;
mod items;
mod statements;

use crate::EcmaError;
use crate::source::EcmaSource;
use crate::style::EcmaStyle;

/// Renders the source model with the canonical domain style.
///
/// - `source`: Semantic source model.
///
/// # Returns
/// Deterministic ECMAScript source text in the canonical style.
///
/// # Errors
/// Returns an error when the model cannot be rendered.
pub fn render(source: &EcmaSource) -> Result<String, EcmaError> {
    render_with_style(source, &EcmaStyle::default())
}

/// Renders the source model with one exact domain style.
///
/// - `source`: Semantic source model.
/// - `style`: Exact presentation choices for indentation and quoting.
///
/// The style moves presentation only: item order, binding names, signatures,
/// and statement tokens are unchanged.
///
/// # Returns
/// Deterministic ECMAScript source text in the requested style.
///
/// # Errors
/// Returns an error when the style is unsupported by the renderer.
pub fn render_with_style(source: &EcmaSource, style: &EcmaStyle) -> Result<String, EcmaError> {
    style.validate().map_err(|reason| EcmaError {
        fatal: true,
        diagnostics: vec![format!("unsupported ECMAScript style: {reason}")],
    })?;
    let mut writer = Writer::new(*style);
    items::write_items(&mut writer, 0, &source.items)?;
    Ok(writer.finish())
}

/// Line-oriented output sink that applies one style.
///
/// # Public members
/// - `new`: Builds a sink for one style.
/// - `style`: Reads the style the sink applies.
/// - `push_line`: Appends one indented line.
/// - `indentation`: Returns the indentation for one depth.
/// - `finish`: Returns the accumulated text.
///
/// # Private members
/// - `style`: Style supplying indentation and quoting.
/// - `text`: Accumulated output.
pub(crate) struct Writer {
    style: EcmaStyle,
    text: String,
}

impl Writer {
    /// Builds an empty sink for one style.
    ///
    /// - `style`: Exact style the sink applies.
    ///
    /// # Returns
    /// The empty sink.
    pub(crate) fn new(style: EcmaStyle) -> Self {
        Self {
            style,
            text: String::new(),
        }
    }

    /// Returns the style the sink applies.
    ///
    /// - `self`: Sink holding the style.
    ///
    /// # Returns
    /// The style used for indentation and quoting.
    pub(crate) fn style(&self) -> EcmaStyle {
        self.style
    }

    /// Appends one line indented to one nesting depth.
    ///
    /// - `self`: Accumulating sink.
    /// - `depth`: Nesting depth of the line.
    /// - `line`: Line text without a trailing newline.
    pub(crate) fn push_line(&mut self, depth: usize, line: &str) {
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(&self.indentation(depth));
        self.text.push_str(line);
    }

    /// Returns the indentation prefix for one nesting depth.
    ///
    /// - `self`: Sink holding the style.
    /// - `depth`: Nesting depth to indent to.
    ///
    /// # Returns
    /// The exact indentation text for that depth.
    pub(crate) fn indentation(&self, depth: usize) -> String {
        self.style.indentation().repeat(depth)
    }

    /// Returns the accumulated text.
    ///
    /// - `self`: Sink to consume.
    ///
    /// # Returns
    /// Every line written so far, joined by newlines.
    pub(crate) fn finish(self) -> String {
        self.text
    }
}
