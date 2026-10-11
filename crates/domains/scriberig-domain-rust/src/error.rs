//! Defines typed errors for Rust source parsing and rendering.

/// Reports invalid Rust syntax adapter input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustSyntaxError {
    /// The parser rejected source text.
    Parse(String),
    /// Rendering received an unsupported semantic item.
    Unsupported(String),
}

impl std::fmt::Display for RustSyntaxError {
    /// Formats the parser error.
    ///
    /// # Parameters
    /// - `self`: Error to format.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns an error when the destination rejects the write.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(message) => write!(formatter, "parse error: {message}"),
            Self::Unsupported(message) => write!(formatter, "unsupported syntax: {message}"),
        }
    }
}
impl std::error::Error for RustSyntaxError {}

/// Reports an analysis failure without invalidating canonical syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustAnalysisError {
    /// Stable failure code.
    pub code: String,
    /// Safe human-readable explanation.
    pub message: String,
}

impl std::fmt::Display for RustAnalysisError {
    /// Formats the analysis failure.
    ///
    /// # Parameters
    /// - `self`: Failure to format.
    /// - `formatter`: Output formatter.
    ///
    /// # Returns
    /// Formatting result.
    ///
    /// # Errors
    /// Returns an error when formatting fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for RustAnalysisError {}
