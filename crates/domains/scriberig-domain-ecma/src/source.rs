//! Defines the semantic source model for the supported ECMAScript subset.
//!
//! The model is parser-independent: it is produced by `crate::extract` from the
//! `oxc` syntax tree and consumed by `crate::render` under an
//! [`crate::style::EcmaStyle`].
//!
//! Supported constructs, and the only constructs the model can hold:
//!
//! - top-level `import` declarations: bare, default, and named with optional
//!   `as` aliases;
//! - `export` applied to a function, variable, or class declaration;
//! - `function` declarations with plain identifier parameters;
//! - `const`, `let`, and `var` declarations whose initializers are literals,
//!   identifiers, calls, or simple arrow functions;
//! - expression statements limited to calls, identifiers, member accesses, and
//!   `=` assignment;
//! - `if` statements with blocks, `else` blocks, and `else if` chains;
//! - `return` with or without a value;
//! - `class` declarations whose members are methods with statement bodies.
//!
//! Anything else is refused with a structured [`crate::EcmaError`] naming the
//! construct instead of being approximated or dropped: TypeScript type
//! annotations, type-only imports and exports, namespaces, decorators, `export
//! default`, `export` specifiers and re-exports, namespace imports, quoted
//! import names such as `import { "a-b" as a } from "m"`, generators,
//! async functions, classes with heritage or property members, computed or
//! private members, object, array, template, regexp, and bigint literals,
//! compound assignment, `this`, `new`, spread, optional chaining, loops, `try`,
//! `switch`, `throw`, labels, `await`, `yield`, directives, and comments.

/// A parsed ECMAScript module restricted to the supported subset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaSource {
    /// Top-level items in source order.
    pub items: Vec<EcmaItem>,
}

/// One top-level item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EcmaItem {
    /// A bare, default, or named `import` declaration.
    Import(EcmaImport),
    /// A declaration introduced by `export`.
    Exported(EcmaDeclaration),
    /// A declaration without a module modifier.
    Declaration(EcmaDeclaration),
    /// A top-level statement outside any declaration body.
    Statement(EcmaStatement),
}

/// A declaration that may be exported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EcmaDeclaration {
    /// A `function` declaration.
    Function(EcmaFunction),
    /// A `const`, `let`, or `var` declaration.
    Variable(EcmaVariable),
    /// A `class` declaration.
    Class(EcmaClass),
}

/// One imported module specifier and the bindings it introduces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaImport {
    /// Module specifier text, without quotes.
    pub source: String,
    /// Local name bound to the module's default export, when present.
    pub default: Option<String>,
    /// Named bindings, in source order.
    pub named: Vec<EcmaImportedName>,
}

/// One `import { imported as local }` binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaImportedName {
    /// Exported name read from the module.
    pub imported: String,
    /// Local alias, when the binding uses `as`.
    pub local: Option<String>,
}

/// A `function` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaFunction {
    /// Function name.
    pub name: String,
    /// Parameters in declaration order.
    pub parameters: Vec<EcmaParameter>,
    /// Body statements in source order.
    pub body: Vec<EcmaStatement>,
}

/// One plain identifier parameter of a function, method, or arrow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaParameter {
    /// Parameter binding name.
    pub name: String,
}

/// A `class` declaration restricted to method members.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaClass {
    /// Class name.
    pub name: String,
    /// Members in source order.
    pub members: Vec<EcmaMethod>,
}

/// One class method or constructor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaMethod {
    /// Method name as written in the class body.
    pub name: String,
    /// Parameters in declaration order.
    pub parameters: Vec<EcmaParameter>,
    /// Body statements in source order.
    pub body: Vec<EcmaStatement>,
}

/// A `const`, `let`, or `var` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaVariable {
    /// Declaration keyword family.
    pub kind: EcmaVariableKind,
    /// Bindings in source order.
    pub bindings: Vec<EcmaBinding>,
}

/// Declaration keyword family of a variable declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EcmaVariableKind {
    /// `var` declaration.
    Var,
    /// `let` declaration.
    Let,
    /// `const` declaration.
    Const,
}

/// One binding of a variable declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaBinding {
    /// Bound identifier name.
    pub name: String,
    /// Initializer expression, when the declaration provides one.
    pub initializer: Option<EcmaExpression>,
}

/// One statement inside a function, method, arrow, or block body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EcmaStatement {
    /// A `const`, `let`, or `var` declaration.
    Variable(EcmaVariable),
    /// A statement whose value is evaluated and discarded.
    Expression(EcmaExpression),
    /// A `return` statement.
    Return(EcmaReturn),
    /// An `if` statement.
    If(EcmaIf),
    /// A bare block statement.
    Block(Vec<EcmaStatement>),
}

/// A `return` statement with an optional value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaReturn {
    /// Returned expression, when the statement has a value.
    pub value: Option<EcmaExpression>,
}

/// An `if` statement with an optional `else` branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaIf {
    /// Condition expression.
    pub test: EcmaExpression,
    /// Statements of the consequent block.
    pub consequent: Vec<EcmaStatement>,
    /// `else` branch, when present.
    pub alternate: Option<EcmaAlternate>,
}

/// The `else` branch of an `if` statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EcmaAlternate {
    /// A nested `if` written without braces.
    ElseIf(Box<EcmaIf>),
    /// An `else { }` block.
    Block(Vec<EcmaStatement>),
}

/// One expression of the supported subset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EcmaExpression {
    /// An identifier reference.
    Identifier(String),
    /// A literal value.
    Literal(EcmaLiteral),
    /// A `object.property` member access.
    Member {
        /// Receiver expression.
        object: Box<EcmaExpression>,
        /// Property name.
        property: String,
    },
    /// A `callee(args)` call.
    Call {
        /// Called expression.
        callee: Box<EcmaExpression>,
        /// Arguments in source order.
        arguments: Vec<EcmaExpression>,
    },
    /// A `target = value` assignment.
    Assignment {
        /// Assignment target, an identifier or member access.
        target: Box<EcmaExpression>,
        /// Assigned expression.
        value: Box<EcmaExpression>,
    },
    /// An arrow function.
    Arrow(EcmaArrow),
}

/// A literal value of the supported subset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EcmaLiteral {
    /// Numeric literal written with its source digits.
    Number(String),
    /// String literal holding its decoded value.
    String(String),
    /// `true` or `false`.
    Boolean(bool),
    /// `null`.
    Null,
}

/// An arrow function within an expression position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaArrow {
    /// Parameters in declaration order.
    pub parameters: Vec<EcmaParameter>,
    /// Arrow body.
    pub body: EcmaArrowBody,
}

/// The body of an arrow function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EcmaArrowBody {
    /// A single expression.
    Expression(Box<EcmaExpression>),
    /// A braced statement list.
    Block(Vec<EcmaStatement>),
}
