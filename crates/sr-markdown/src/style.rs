//! Defines immutable, deterministic Markdown formatting options.

/// Formatting choices that do not alter Markdown semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownStyle {
    /// Marker used for unordered list items.
    pub list_marker: char,
    /// Delimiter used for ordered list items.
    pub ordered_list_delimiter: char,
    /// Number of blank lines between blocks.
    pub blank_lines_between_blocks: usize,
    /// Whether output ends with a newline.
    pub final_newline: bool,
    /// Fence used for code blocks.
    pub code_fence_preference: String,
    /// Padding inserted around table cell separators.
    pub table_spacing: usize,
}

impl Default for MarkdownStyle {
    /// Creates the canonical Markdown formatting style.
    ///
    /// # Returns
    /// The default deterministic style.
    fn default() -> Self {
        Self {
            list_marker: '-',
            ordered_list_delimiter: '.',
            blank_lines_between_blocks: 1,
            final_newline: true,
            code_fence_preference: "```".to_owned(),
            table_spacing: 1,
        }
    }
}

impl MarkdownStyle {
    /// Validates style choices and returns a normalized immutable style.
    ///
    /// # Returns
    /// The validated style.
    ///
    /// # Errors
    /// Returns an error when a marker, fence, or spacing value is unsupported.
    pub fn validate(self) -> Result<Self, String> {
        if !matches!(self.list_marker, '-' | '*' | '+') {
            return Err("unordered list marker must be '-', '*' or '+'".to_owned());
        }
        if !matches!(self.ordered_list_delimiter, '.' | ')') {
            return Err("ordered list delimiter must be '.' or ')'".to_owned());
        }
        if self.blank_lines_between_blocks > 2 {
            return Err("at most two blank lines are supported".to_owned());
        }
        if self.code_fence_preference.len() < 3
            || !self.code_fence_preference.chars().all(|value| value == '`')
        {
            return Err("code fence must contain at least three backticks".to_owned());
        }
        if self.table_spacing > 4 {
            return Err("table spacing is limited to four spaces".to_owned());
        }
        Ok(self)
    }
}
