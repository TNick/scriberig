//! Declares why one projection is being produced, independent of any domain.

use std::fmt;

/// Names the audience and mutability contract of one projection computation.
///
/// Purpose is domain-neutral. It exists so style resolution can decide which
/// sources may legitimately apply without inspecting style bytes or product
/// surfaces. A personal interactive view must never silently share its style
/// with workspace materialization, and materialization must never silently
/// borrow a personal preference.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProjectionPurpose {
    /// A personal interactive view for one principal and one view session.
    InteractiveView,
    /// Shared materialization of canonical state into an external workspace.
    SharedMaterialization,
    /// Explicit export where the caller named every non-default style source.
    ExplicitExport,
}

impl ProjectionPurpose {
    /// Reports whether personal principal preference may apply at all.
    ///
    /// - `self`: Purpose being consulted.
    ///
    /// True only for purposes that are explicitly personal. Shared
    /// materialization stays false so a workspace byte stream never depends on
    /// whose preference happened to be loaded.
    #[must_use]
    /// # Returns
    /// Whether personal preference sources are permitted.
    pub fn admits_personal_preference(self) -> bool {
        matches!(self, Self::InteractiveView | Self::ExplicitExport)
    }

    /// Reports whether a personal source must have been named by the caller.
    ///
    /// - `self`: Purpose being consulted.
    ///
    /// True for explicit export, where ambient view or preference state may not
    /// be borrowed invisibly.
    #[must_use]
    /// # Returns
    /// Whether personal sources require explicit caller selection.
    pub fn requires_explicit_personal_source(self) -> bool {
        matches!(self, Self::ExplicitExport)
    }
}

impl fmt::Display for ProjectionPurpose {
    /// Formats the purpose as a stable lower-case label.
    ///
    /// - `self`: Purpose to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::InteractiveView => "interactive-view",
            Self::SharedMaterialization => "shared-materialization",
            Self::ExplicitExport => "explicit-export",
        };
        formatter.write_str(label)
    }
}
