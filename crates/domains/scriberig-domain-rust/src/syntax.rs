//! Defines parser-independent Rust syntax values.

use crate::comments::RustComment;

/// Parsed Rust source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustFile {
    /// Top-level declarations in source order.
    pub items: Vec<RustItem>,
    /// Source-authored comments.
    pub comments: Vec<RustComment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Represents a parsed top-level item in a Rust source module.
pub enum RustItem {
    /// Function declaration with parsed signature and body syntax.
    Function(RustFunction),
    /// Structure declaration with parsed semantic members.
    Struct(RustDeclaration),
    /// Enumeration declaration with parsed variants.
    Enum(RustDeclaration),
    /// Implementation block.
    Impl(RustDeclaration),
    /// Macro invocation retaining token syntax.
    Macro(RustMacro),
    /// Other supported declaration retained as canonical tokens.
    Other(String),
}

/// Rust function syntax relevant to source semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustFunction {
    /// Function name.
    pub name: String,
    /// Attribute token strings.
    pub attributes: Vec<String>,
    /// Parameter patterns and type syntax.
    pub parameters: Vec<RustParameter>,
    /// Function body statements.
    pub body: RustBlock,
}

/// Function parameter preserving pattern and type syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustParameter {
    /// Original pattern tokens.
    pub pattern: String,
    /// Type syntax tokens.
    pub ty: String,
}

/// A structured block with source-level statement tokens.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustBlock {
    /// Statements in source order.
    pub statements: Vec<RustStatement>,
}

/// One statement or expression statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustStatement {
    /// Canonical token representation.
    pub tokens: String,
    /// Whether the statement is an expression.
    pub expression: bool,
}

/// Struct, enum, or implementation declaration retained as tokens.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustDeclaration {
    /// Declaration token representation.
    pub tokens: String,
}

/// Macro invocation with unexpanded token tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustMacro {
    /// Macro path.
    pub path: String,
    /// Delimited token tree.
    pub tokens: String,
}
