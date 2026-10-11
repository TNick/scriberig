//! Maps parsed Rust items to stable adapter-level kinds.

/// Stable kind assigned to a parsed Rust item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustItemKind {
    /// Function declaration.
    Function,
    /// Struct declaration.
    Struct,
    /// Enum declaration.
    Enum,
    /// Implementation block.
    Impl,
    /// Macro invocation.
    Macro,
    /// Other declaration.
    Other,
}
