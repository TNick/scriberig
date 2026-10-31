//! Defines safe XLSX adapter failures.

/// Reports invalid XLSX input or an unsupported semantic cell edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XlsxAdapterError {
    /// Stable diagnostic category.
    pub code: &'static str,
    /// Safe adapter diagnostic.
    pub message: String,
}

impl std::fmt::Display for XlsxAdapterError {
    /// Formats the XLSX adapter failure.
    ///
    /// # Parameters
    /// - `self`: Adapter error being formatted.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// Success after writing the stable category and safe message.
    ///
    /// # Errors
    /// Returns the destination's formatting error if writing fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for XlsxAdapterError {}
