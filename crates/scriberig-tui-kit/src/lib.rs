//! Exposes Scriberig-owned adapters for reusable terminal presentation.

mod composer;
mod markdown;
mod policy;
mod selection;
mod text;
mod theme;

pub use composer::{Composer, CursorMotion};
pub use markdown::render_markdown;
pub use policy::{ClipboardPolicy, TerminalPolicy};
pub use selection::{SelectionOption, StableSelector};
pub use text::{ExternalText, TextBlock, display_width};
pub use theme::{PaletteColor, StyleToken, Tone, style_for};
