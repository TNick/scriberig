//! Maps parsed Rust items to stable adapter-level kinds.

/// Stable kind assigned to a parsed Rust item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustItemKind {
    /// Function declaration.
    Function,
    /// Structure declaration.
    Struct,
    /// Enumeration declaration.
    Enum,
    /// Implementation block.
    Impl,
    /// Macro invocation.
    Macro,
    /// Other supported declaration kind.
    Other,
}
