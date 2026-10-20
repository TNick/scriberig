//! Maps semantic presentation tones to bounded terminal style tokens.

/// Names a semantic tone used by reusable terminal views.
///
/// # Public members
/// - `Text`: Ordinary foreground text.
/// - `Muted`: Secondary text.
/// - `Accent`: Emphasized text.
/// - `Success`: Successful operation state.
/// - `Warning`: Warning state.
/// - `Error`: Error state.
/// - `BranchDepth`: Structural ancestry depth, mapped through a finite palette.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tone {
    /// Ordinary foreground text.
    Text,
    /// Secondary text.
    Muted,
    /// Emphasized text.
    Accent,
    /// Successful operation state.
    Success,
    /// Warning state.
    Warning,
    /// Error state.
    Error,
    /// Structural ancestry depth.
    BranchDepth(usize),
}

/// Describes a safe terminal foreground palette value.
///
/// # Public members
/// - `Default`: Uses the terminal default foreground.
/// - `Ansi256`: Uses a color from the ANSI 256-color palette.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteColor {
    /// Uses the terminal default foreground.
    Default,
    /// Uses a color from the ANSI 256-color palette.
    Ansi256(u8),
}

/// Provides a Scriberig-owned style token for a semantic tone.
///
/// # Public members
/// - `foreground`: Bounded palette color.
/// - `bold`: Whether the text should be emphasized in bold.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StyleToken {
    /// Bounded palette color.
    pub foreground: PaletteColor,
    /// Whether the text should be emphasized in bold.
    pub bold: bool,
}

/// Maps a semantic tone to a constrained style token.
///
/// # Parameters
/// - `tone`: Semantic purpose of the text.
///
/// # Returns
/// A style mapping containing only adapter-owned terminal values.
#[must_use]
pub const fn style_for(tone: Tone) -> StyleToken {
    match tone {
        Tone::Text => StyleToken {
            foreground: PaletteColor::Default,
            bold: false,
        },
        Tone::Muted => StyleToken {
            foreground: PaletteColor::Ansi256(244),
            bold: false,
        },
        Tone::Accent => StyleToken {
            foreground: PaletteColor::Ansi256(75),
            bold: true,
        },
        Tone::Success => StyleToken {
            foreground: PaletteColor::Ansi256(40),
            bold: false,
        },
        Tone::Warning => StyleToken {
            foreground: PaletteColor::Ansi256(214),
            bold: true,
        },
        Tone::Error => StyleToken {
            foreground: PaletteColor::Ansi256(196),
            bold: true,
        },
        Tone::BranchDepth(depth) => {
            let colors = [39, 40, 220, 201, 33, 196];
            StyleToken {
                foreground: PaletteColor::Ansi256(colors[depth % colors.len()]),
                bold: false,
            }
        }
    }
}
