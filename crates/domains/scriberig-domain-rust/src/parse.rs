//! Parses Rust source into parser-independent syntax values.

use crate::{
    comments::recover_comments,
    error::RustSyntaxError,
    syntax::{
        RustBlock, RustDeclaration, RustFile, RustFunction, RustItem, RustMacro, RustParameter,
        RustStatement,
    },
};
use quote::ToTokens;
use syn::{Item, Stmt};

/// Parses Rust source while keeping parser AST types private.
/// # Parameters
/// - `source`: Rust source text.
/// # Returns
/// Structured Rust syntax and source comments.
/// # Errors
/// Returns a typed parse error for invalid Rust source.
pub fn parse(source: &str) -> Result<RustFile, RustSyntaxError> {
    let file =
        syn::parse_file(source).map_err(|error| RustSyntaxError::Parse(error.to_string()))?;
    let items = file.items.into_iter().map(map_item).collect();
    Ok(RustFile {
        items,
        comments: recover_comments(source),
    })
}

/// Converts one private parser item to adapter syntax.
/// # Parameters
/// - `item`: Parser item.
/// # Returns
/// Parser-independent syntax.
fn map_item(item: Item) -> RustItem {
    match item {
        Item::Fn(function) => RustItem::Function(RustFunction {
            name: function.sig.ident.to_string(),
            attributes: function
                .attrs
                .iter()
                .map(ToTokens::to_token_stream)
                .map(|tokens| tokens.to_string())
                .collect(),
            parameters: function
                .sig
                .inputs
                .iter()
                .filter_map(|input| match input {
                    syn::FnArg::Typed(value) => Some(RustParameter {
                        pattern: value.pat.to_token_stream().to_string(),
                        ty: value.ty.to_token_stream().to_string(),
                    }),
                    syn::FnArg::Receiver(_) => None,
                })
                .collect(),
            body: RustBlock {
                statements: function.block.stmts.iter().map(map_statement).collect(),
            },
        }),
        Item::Struct(value) => RustItem::Struct(RustDeclaration {
            tokens: value.to_token_stream().to_string(),
        }),
        Item::Enum(value) => RustItem::Enum(RustDeclaration {
            tokens: value.to_token_stream().to_string(),
        }),
        Item::Impl(value) => RustItem::Impl(RustDeclaration {
            tokens: value.to_token_stream().to_string(),
        }),
        Item::Macro(value) => RustItem::Macro(RustMacro {
            path: value.mac.path.to_token_stream().to_string(),
            tokens: value.mac.tokens.to_string(),
        }),
        other => RustItem::Other(other.to_token_stream().to_string()),
    }
}
/// Converts one private parser statement to adapter syntax.
/// # Parameters
/// - `statement`: Parser statement.
/// # Returns
/// Parser-independent statement syntax.
fn map_statement(statement: &Stmt) -> RustStatement {
    RustStatement {
        tokens: statement.to_token_stream().to_string(),
        expression: matches!(statement, Stmt::Expr(_, _) | Stmt::Macro(_)),
    }
}
