//! Defines modal presentation state and its focused overlay models.

pub mod help;
pub mod input;
pub mod palette;

/// Explicit review confirmation details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmationState {
    /// Action that will be invoked after confirmation.
    pub action: String,
    /// Semantic target or change handle.
    pub target: String,
    /// Exact reviewed base context.
    pub base: String,
    /// Short risk/change summary.
    pub summary: String,
    /// Exact canonical command held until explicit confirmation.
    pub commit: crate::view::changes::ConfirmedCommit,
}

/// The single active modal overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Overlay {
    /// Keyboard help generated from the keymap.
    Help,
    /// Local and semantic command palette.
    Palette(palette::PaletteState),
    /// Text input used by find and future commands.
    Input(input::InputState),
    /// Confirmation modal for a reviewed state-changing operation.
    Confirm(ConfirmationState),
}
