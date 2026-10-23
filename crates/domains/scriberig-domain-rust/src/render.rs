//! Renders parser-independent Rust syntax deterministically under a domain style.

use crate::{
    error::RustSyntaxError,
    style::RustStyle,
    syntax::{RustFile, RustItem},
};

/// Renders supported Rust syntax with the canonical domain style.
///
/// - `file`: Structured Rust syntax.
///
/// # Returns
/// Deterministic Rust source text in the canonical style.
///
/// # Errors
/// Returns an error when an item cannot be rendered.
pub fn render(file: &RustFile) -> Result<String, RustSyntaxError> {
    render_with_style(file, &RustStyle::default())
}

/// Renders supported Rust syntax with one exact domain style.
///
/// - `file`: Structured Rust syntax.
/// - `style`: Exact presentation choices for indentation and item spacing.
///
/// The semantic structures are unchanged by the style: the same item order,
/// signatures, and statement tokens are emitted, only their presentation moves.
///
/// # Returns
/// Deterministic Rust source text in the requested style.
///
/// # Errors
/// Returns an error when the style is unsupported or an item cannot be rendered.
pub fn render_with_style(file: &RustFile, style: &RustStyle) -> Result<String, RustSyntaxError> {
    style.validate().map_err(RustSyntaxError::Unsupported)?;
    let separator = "\n".repeat(1 + style.blank_lines_between_items);
    let indent = style.indentation();
    let mut output = String::new();
    for item in &file.items {
        if !output.is_empty() {
            output.push_str(&separator);
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
                    output.push_str(&indent);
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
