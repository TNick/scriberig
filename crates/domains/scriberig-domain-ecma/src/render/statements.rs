//! Renders statements of function, method, arrow, and block bodies.

use crate::EcmaError;
use crate::render::Writer;
use crate::render::expressions::expression_text;
use crate::render::items::variable_text;
use crate::source::{EcmaAlternate, EcmaIf, EcmaReturn, EcmaStatement};
use crate::style::EcmaStyle;

/// Writes one statement list in source order.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the statements.
/// - `body`: Statements to render.
///
/// # Returns
/// Unit once every statement is written.
///
/// # Errors
/// Returns an error when a statement cannot be rendered.
pub(crate) fn write_body(
    writer: &mut Writer,
    depth: usize,
    body: &[EcmaStatement],
) -> Result<(), EcmaError> {
    for statement in body {
        write_statement(writer, depth, statement)?;
    }
    Ok(())
}

/// Writes one statement.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the statement.
/// - `statement`: Statement to render.
///
/// # Returns
/// Unit once the statement is written.
///
/// # Errors
/// Returns an error when the statement cannot be rendered.
pub(crate) fn write_statement(
    writer: &mut Writer,
    depth: usize,
    statement: &EcmaStatement,
) -> Result<(), EcmaError> {
    match statement {
        EcmaStatement::Variable(value) => {
            let text = variable_text(value, "", writer.style())?;
            writer.push_line(depth, &text);
            Ok(())
        }
        EcmaStatement::Expression(value) => {
            let text = expression_text(value, depth, writer.style())?;
            writer.push_line(depth, &format!("{text};"));
            Ok(())
        }
        EcmaStatement::Return(value) => {
            writer.push_line(depth, &return_text(value, depth, writer.style())?);
            Ok(())
        }
        EcmaStatement::If(value) => write_if(writer, depth, value),
        EcmaStatement::Block(body) => {
            if body.is_empty() {
                writer.push_line(depth, "{}");
                return Ok(());
            }
            writer.push_line(depth, "{");
            write_body(writer, depth + 1, body)?;
            writer.push_line(depth, "}");
            Ok(())
        }
    }
}

/// Renders one `return` statement.
///
/// - `value`: Return statement to render.
/// - `depth`: Nesting depth, used to indent nested arrow bodies.
/// - `style`: Style supplying indentation and quoting.
///
/// # Returns
/// The single-line `return` statement.
///
/// # Errors
/// Returns an error when the returned expression cannot be rendered.
fn return_text(value: &EcmaReturn, depth: usize, style: EcmaStyle) -> Result<String, EcmaError> {
    match &value.value {
        Some(expression) => Ok(format!(
            "return {};",
            expression_text(expression, depth, style)?
        )),
        None => Ok("return;".to_owned()),
    }
}

/// Writes one `if` statement and its `else` chain.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the statement.
/// - `value`: `if` statement to render.
///
/// # Returns
/// Unit once the statement is written.
///
/// # Errors
/// Returns an error when a branch or condition cannot be rendered.
fn write_if(writer: &mut Writer, depth: usize, value: &EcmaIf) -> Result<(), EcmaError> {
    let test = expression_text(&value.test, depth, writer.style())?;
    writer.push_line(depth, &format!("if ({test}) {{"));
    write_body(writer, depth + 1, &value.consequent)?;
    write_alternate(writer, depth, value.alternate.as_ref())
}

/// Writes the closing brace of an `if` statement and its optional `else` branch.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the enclosing `if` statement.
/// - `alternate`: `else` branch, when present.
///
/// # Returns
/// Unit once the branch is written.
///
/// # Errors
/// Returns an error when the branch cannot be rendered.
fn write_alternate(
    writer: &mut Writer,
    depth: usize,
    alternate: Option<&EcmaAlternate>,
) -> Result<(), EcmaError> {
    match alternate {
        None => {
            writer.push_line(depth, "}");
            Ok(())
        }
        Some(EcmaAlternate::Block(body)) => {
            writer.push_line(depth, "} else {");
            write_body(writer, depth + 1, body)?;
            writer.push_line(depth, "}");
            Ok(())
        }
        Some(EcmaAlternate::ElseIf(nested)) => {
            let test = expression_text(&nested.test, depth, writer.style())?;
            writer.push_line(depth, &format!("}} else if ({test}) {{"));
            write_body(writer, depth + 1, &nested.consequent)?;
            write_alternate(writer, depth, nested.alternate.as_ref())
        }
    }
}
