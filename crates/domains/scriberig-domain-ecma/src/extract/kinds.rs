//! Names the syntax nodes this domain refuses so errors stay structured.
//!
//! Each function maps one `oxc` AST variant to the construct name reported in a
//! refusal diagnostic. The matches are exhaustive on purpose: adding a variant
//! to `oxc` breaks this file instead of silently producing a vague error.

use oxc_ast::ast::{BindingPattern, ClassElement, Declaration, Expression, Statement};

/// Names one statement variant for a refusal diagnostic.
///
/// - `statement`: Statement the modeler cannot represent.
///
/// # Returns
/// The construct name reported to the caller.
pub(crate) fn statement_kind(statement: &Statement<'_>) -> &'static str {
    match statement {
        Statement::BlockStatement(_) => "block statement",
        Statement::BreakStatement(_) => "break statement",
        Statement::ContinueStatement(_) => "continue statement",
        Statement::DebuggerStatement(_) => "debugger statement",
        Statement::DoWhileStatement(_) => "do-while statement",
        Statement::EmptyStatement(_) => "empty statement",
        Statement::ExpressionStatement(_) => "expression statement",
        Statement::ForInStatement(_) => "for-in statement",
        Statement::ForOfStatement(_) => "for-of statement",
        Statement::ForStatement(_) => "for statement",
        Statement::IfStatement(_) => "if statement",
        Statement::LabeledStatement(_) => "labeled statement",
        Statement::ReturnStatement(_) => "return statement",
        Statement::SwitchStatement(_) => "switch statement",
        Statement::ThrowStatement(_) => "throw statement",
        Statement::TryStatement(_) => "try statement",
        Statement::WhileStatement(_) => "while statement",
        Statement::WithStatement(_) => "with statement",
        Statement::VariableDeclaration(_) => "variable declaration",
        Statement::FunctionDeclaration(_) => "function declaration",
        Statement::ClassDeclaration(_) => "class declaration",
        Statement::TSTypeAliasDeclaration(_) => "type alias declaration",
        Statement::TSInterfaceDeclaration(_) => "interface declaration",
        Statement::TSEnumDeclaration(_) => "enum declaration",
        Statement::TSExternalModuleDeclaration(_) => "external module declaration",
        Statement::TSNamespaceDeclaration(_) => "namespace declaration",
        Statement::TSGlobalDeclaration(_) => "global declaration",
        Statement::TSImportEqualsDeclaration(_) => "import equals declaration",
        Statement::TSExportAssignment(_) => "export assignment",
        Statement::TSNamespaceExportDeclaration(_) => "namespace export declaration",
        Statement::ImportDeclaration(_) => "import declaration",
        Statement::ExportAllDeclaration(_) => "export all declaration",
        Statement::ExportDefaultDeclaration(_) => "export default declaration",
        Statement::ExportDeclaration(_) => "export declaration",
        Statement::ExportNamedDeclaration(_) => "export specifier list",
        Statement::ExportFromDeclaration(_) => "export re-export declaration",
    }
}

/// Names one expression variant for a refusal diagnostic.
///
/// - `expression`: Expression the modeler cannot represent.
///
/// # Returns
/// The construct name reported to the caller.
pub(crate) fn expression_kind(expression: &Expression<'_>) -> &'static str {
    match expression {
        Expression::BooleanLiteral(_) => "boolean literal",
        Expression::NullLiteral(_) => "null literal",
        Expression::NumericLiteral(_) => "numeric literal",
        Expression::BigIntLiteral(_) => "bigint literal",
        Expression::RegExpLiteral(_) => "regular expression literal",
        Expression::StringLiteral(_) => "string literal",
        Expression::TemplateLiteral(_) => "template literal",
        Expression::Identifier(_) => "identifier",
        Expression::ComputedMemberExpression(_) => "computed member expression",
        Expression::StaticMemberExpression(_) => "static member expression",
        Expression::PrivateFieldExpression(_) => "private field expression",
        Expression::Super(_) => "super",
        Expression::ArrayExpression(_) => "array expression",
        Expression::ArrowFunctionExpression(_) => "arrow function expression",
        Expression::AssignmentExpression(_) => "assignment expression",
        Expression::AwaitExpression(_) => "await expression",
        Expression::BinaryExpression(_) => "binary expression",
        Expression::CallExpression(_) => "call expression",
        Expression::ChainExpression(_) => "optional chain expression",
        Expression::ClassExpression(_) => "class expression",
        Expression::ConditionalExpression(_) => "conditional expression",
        Expression::FunctionExpression(_) => "function expression",
        Expression::ImportExpression(_) => "dynamic import expression",
        Expression::LogicalExpression(_) => "logical expression",
        Expression::NewExpression(_) => "new expression",
        Expression::ObjectExpression(_) => "object expression",
        Expression::ParenthesizedExpression(_) => "parenthesized expression",
        Expression::SequenceExpression(_) => "sequence expression",
        Expression::TaggedTemplateExpression(_) => "tagged template expression",
        Expression::ThisExpression(_) => "this expression",
        Expression::UnaryExpression(_) => "unary expression",
        Expression::UpdateExpression(_) => "update expression",
        Expression::YieldExpression(_) => "yield expression",
        Expression::PrivateInExpression(_) => "private in expression",
        Expression::ImportMeta(_) => "import meta",
        Expression::NewTarget(_) => "new target",
        Expression::JSXElement(_) => "JSX element",
        Expression::JSXFragment(_) => "JSX fragment",
        Expression::TSAsExpression(_) => "as expression",
        Expression::TSSatisfiesExpression(_) => "satisfies expression",
        Expression::TSTypeAssertion(_) => "type assertion expression",
        Expression::TSNonNullExpression(_) => "non-null expression",
        Expression::TSInstantiationExpression(_) => "instantiation expression",
        Expression::V8IntrinsicExpression(_) => "V8 intrinsic expression",
    }
}

/// Names one binding pattern variant for a refusal diagnostic.
///
/// - `pattern`: Binding pattern the modeler cannot represent.
///
/// # Returns
/// The construct name reported to the caller.
pub(crate) fn binding_kind(pattern: &BindingPattern<'_>) -> &'static str {
    match pattern {
        BindingPattern::BindingIdentifier(_) => "binding identifier",
        BindingPattern::ObjectPattern(_) => "object binding pattern",
        BindingPattern::ArrayPattern(_) => "array binding pattern",
        BindingPattern::AssignmentPattern(_) => "default binding pattern",
    }
}

/// Names one declaration variant for a refusal diagnostic.
///
/// - `declaration`: Declaration the modeler cannot represent.
///
/// # Returns
/// The construct name reported to the caller.
pub(crate) fn declaration_kind(declaration: &Declaration<'_>) -> &'static str {
    match declaration {
        Declaration::VariableDeclaration(_) => "variable declaration",
        Declaration::FunctionDeclaration(_) => "function declaration",
        Declaration::ClassDeclaration(_) => "class declaration",
        Declaration::TSTypeAliasDeclaration(_) => "type alias declaration",
        Declaration::TSInterfaceDeclaration(_) => "interface declaration",
        Declaration::TSEnumDeclaration(_) => "enum declaration",
        Declaration::TSExternalModuleDeclaration(_) => "external module declaration",
        Declaration::TSNamespaceDeclaration(_) => "namespace declaration",
        Declaration::TSGlobalDeclaration(_) => "global declaration",
        Declaration::TSImportEqualsDeclaration(_) => "import equals declaration",
    }
}

/// Names one class member variant for a refusal diagnostic.
///
/// - `member`: Class member the modeler cannot represent.
///
/// # Returns
/// The construct name reported to the caller.
pub(crate) fn class_element_kind(member: &ClassElement<'_>) -> &'static str {
    match member {
        ClassElement::StaticBlock(_) => "class static block",
        ClassElement::MethodDefinition(_) => "class method definition",
        ClassElement::PropertyDefinition(_) => "class property definition",
        ClassElement::AccessorProperty(_) => "class accessor property",
        ClassElement::TSIndexSignature(_) => "class index signature",
    }
}
