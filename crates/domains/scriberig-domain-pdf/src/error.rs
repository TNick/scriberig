//! Defines errors returned by the PDF adapter.

/// Reports malformed or unsupported PDF import input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdfError {
    EmptySource,
    InvalidHeader,
}

impl std::fmt::Display for PdfError {
    /// Formats the PDF error.
    ///
    /// - `self`: Error to format.
    /// - `f`: Formatter destination.
    ///
    /// Formatting result.
    ///
    /// Returns a formatting error when the destination rejects output.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PdfError {}
