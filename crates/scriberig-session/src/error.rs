//! Defines session validation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub enum SessionError {
    InvalidTransition {
        from: crate::BranchStatus,
        to: crate::BranchStatus,
    },
    EmptyTitle,
    MissingParent,
    DuplicateBranch,
}

impl std::fmt::Display for SessionError {
    /// Formats a session error.
    /// Executes `fmt` for structured session state.
    ///
    /// # Parameters
    /// - `self`: Session value or formatter.
    /// - `id`: Stable entity identifier when applicable.
    /// - `title`: Branch title when applicable.
    /// - `next`: Requested lifecycle state when applicable.
    /// - `turn`: Turn to append when applicable.
    /// - `entity`: Cross-domain entity to reference when applicable.
    /// - `session`: Session snapshot when applicable.
    /// - `branch`: Branch snapshot when applicable.
    /// - `decisions`: Decision entries when applicable.
    /// - `f`: Formatting destination when applicable.
    ///
    /// # Returns
    /// The structured result.
    ///
    /// # Errors
    /// Returns a validation or formatting error when inputs are invalid.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SessionError {}
