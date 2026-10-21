//! Defines explicit policy for terminal capabilities with extra authority.

/// Describes whether a clipboard action followed an explicit user action.
///
/// # Public members
/// - `Disabled`: Disallows clipboard use.
/// - `ExplicitUserAction`: Allows use only after a user action.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardPolicy {
    /// Disallows clipboard access.
    Disabled,
    /// Allows clipboard access only for an explicit user action.
    ExplicitUserAction,
}

/// States the conservative defaults for optional terminal capabilities.
///
/// # Public members
/// - [`TerminalPolicy::images_enabled`]: Reports whether image protocols are
///   enabled.
/// - [`TerminalPolicy::clipboard_allowed`]: Checks explicit clipboard policy.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalPolicy;

impl TerminalPolicy {
    /// Reports whether terminal image protocols are enabled.
    ///
    /// # Returns
    /// Always false for the initial Scriberig adapter.
    #[must_use]
    pub const fn images_enabled(self) -> bool {
        false
    }

    /// Checks if an explicit user action and policy allow clipboard access.
    ///
    /// # Parameters
    /// - `policy`: Scriberig clipboard policy.
    /// - `user_action`: Whether the caller represents an explicit user action.
    ///
    /// # Returns
    /// True only for explicit actions under `ExplicitUserAction` policy.
    #[must_use]
    pub const fn clipboard_allowed(self, policy: ClipboardPolicy, user_action: bool) -> bool {
        matches!(policy, ClipboardPolicy::ExplicitUserAction) && user_action
    }
}
