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
    /// The continuation branch does not exist.
    MissingAnchorBranch,
    /// The continuation target does not exist in the branch.
    MissingAnchorTarget,
    /// The target exists but does not represent a user-replyable point.
    NonReplyableAnchor,
    /// A plan identity already exists in the session.
    DuplicatePlan,
    /// The plan's owner does not match the containing session.
    PlanSessionMismatch,
    /// The plan has no exact continuation anchor.
    MissingPlanAnchor,
    /// The plan root branch differs from the resolved anchor branch.
    PlanAnchorMismatch,
}

impl std::fmt::Display for SessionError {
    /// Formats a session error.
    /// Executes `fmt` for structured session state.
    ///
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
    /// The structured result.
    ///
    /// Returns a validation or formatting error when inputs are invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SessionError {}
