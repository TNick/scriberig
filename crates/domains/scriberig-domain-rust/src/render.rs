//! Renders parser-independent Rust syntax deterministically.

use crate::{
    error::RustSyntaxError,
    syntax::{RustFile, RustItem},
};

/// Renders supported Rust syntax from semantic structures.
/// - `file`: Structured Rust syntax.
///
/// Deterministic Rust source text.
///
/// Returns an error when an item cannot be rendered.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn render(file: &RustFile) -> Result<String, RustSyntaxError> {
    let mut output = String::new();
    for item in &file.items {
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        match item {
            RustItem::Function(function) => {
                for attribute in &function.attributes {
                    output.push_str(attribute);
                    output.push('\n');
                }
                output.push_str("fn ");
                output.push_str(&function.name);
                output.push('(');
                output.push_str(
                    &function
                        .parameters
                        .iter()
                        .map(|parameter| format!("{}: {}", parameter.pattern, parameter.ty))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                output.push_str(") {");
                for statement in &function.body.statements {
                    output.push('\n');
                    output.push_str("    ");
                    output.push_str(&statement.tokens);
                }
                output.push_str("\n}");
            }
            RustItem::Struct(value) | RustItem::Enum(value) | RustItem::Impl(value) => {
                output.push_str(&value.tokens);
            }
            RustItem::Other(value) => output.push_str(value),
            RustItem::Macro(value) => {
                output.push_str(&value.path);
                output.push('!');
                output.push_str(&value.tokens);
                output.push(';');
            }
        }
    }
    Ok(output)
}
