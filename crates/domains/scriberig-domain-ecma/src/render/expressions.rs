//! Renders the supported expression subset, applying the style's quote choice.

use crate::EcmaError;
use crate::render::Writer;
use crate::render::items::parameter_text;
use crate::render::statements::write_body;
use crate::source::{EcmaArrow, EcmaArrowBody, EcmaExpression, EcmaLiteral};
use crate::style::EcmaStyle;

/// Renders one expression at one nesting depth.
///
/// - `expression`: Expression to render.
/// - `depth`: Nesting depth, used when an arrow has a block body.
/// - `style`: Style supplying indentation and quoting.
///
/// # Returns
/// The expression text.
///
/// # Errors
/// Returns an error when a nested statement cannot be rendered.
pub(crate) fn expression_text(
    expression: &EcmaExpression,
    depth: usize,
    style: EcmaStyle,
) -> Result<String, EcmaError> {
    match expression {
        EcmaExpression::Identifier(name) => Ok(name.clone()),
        EcmaExpression::Literal(value) => Ok(literal_text(value, style)),
        EcmaExpression::Member { object, property } => {
            Ok(format!("{}.{property}", atom_text(object, depth, style)?))
        }
        EcmaExpression::Call { callee, arguments } => {
            let args = arguments
                .iter()
                .map(|argument| expression_text(argument, depth, style))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            Ok(format!("{}({args})", atom_text(callee, depth, style)?))
        }
        EcmaExpression::Assignment { target, value } => Ok(format!(
            "{} = {}",
            atom_text(target, depth, style)?,
            expression_text(value, depth, style)?
        )),
        EcmaExpression::Arrow(value) => arrow_text(value, depth, style),
    }
}

/// Renders one expression in a position that must be a complete atom.
///
/// Arrows and assignments are parenthesized there so the rendered text re-parses
/// to the same model instead of re-associating.
///
/// - `expression`: Expression to render as an atom.
/// - `depth`: Nesting depth, used when an arrow has a block body.
/// - `style`: Style supplying indentation and quoting.
///
/// # Returns
/// The possibly parenthesized expression text.
///
/// # Errors
/// Returns an error when a nested statement cannot be rendered.
fn atom_text(
    expression: &EcmaExpression,
    depth: usize,
    style: EcmaStyle,
) -> Result<String, EcmaError> {
    let text = expression_text(expression, depth, style)?;
    if matches!(
        expression,
        EcmaExpression::Arrow(_) | EcmaExpression::Assignment { .. }
    ) {
        return Ok(format!("({text})"));
    }
    Ok(text)
}

/// Renders one literal with the style's quote choice.
///
/// - `literal`: Literal to render.
/// - `style`: Style supplying the quote character.
///
/// # Returns
/// The literal text.
fn literal_text(literal: &EcmaLiteral, style: EcmaStyle) -> String {
    match literal {
        EcmaLiteral::Number(text) => text.clone(),
        EcmaLiteral::String(value) => quoted(value, style),
        EcmaLiteral::Boolean(value) => (if *value { "true" } else { "false" }).to_owned(),
        EcmaLiteral::Null => "null".to_owned(),
    }
}

/// Quotes one string value with the style's quote character.
///
/// - `value`: String value to render.
/// - `style`: Style supplying the quote character.
///
/// # Returns
/// A string literal whose decoded value is exactly `value`.
pub(crate) fn quoted(value: &str, style: EcmaStyle) -> String {
    let quote = style.quote_character();
    let mut out = String::new();
    out.push(quote);
    for character in value.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                // Control characters are written as a fixed four-digit `\u` escape
                // so the rendered literal always re-parses to the same value.
                let code = c as u32;
                out.push_str("\\u00");
                out.push(char::from_digit((code >> 4) & 0x0f, 16).unwrap_or('0'));
                out.push(char::from_digit(code & 0x0f, 16).unwrap_or('0'));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Renders one arrow function.
///
/// - `arrow`: Arrow function to render.
/// - `depth`: Nesting depth, used for a block body.
/// - `style`: Style supplying indentation and quoting.
///
/// # Returns
/// The arrow function text.
///
/// # Errors
/// Returns an error when a body statement cannot be rendered.
fn arrow_text(arrow: &EcmaArrow, depth: usize, style: EcmaStyle) -> Result<String, EcmaError> {
    let parameters = format!("({})", parameter_text(&arrow.parameters));
    match &arrow.body {
        EcmaArrowBody::Expression(value) => Ok(format!(
            "{parameters} => {}",
            expression_text(value, depth, style)?
        )),
        EcmaArrowBody::Block(body) if body.is_empty() => Ok(format!("{parameters} => {{}}")),
        EcmaArrowBody::Block(body) => {
            let mut nested = Writer::new(style);
            write_body(&mut nested, depth + 1, body)?;
            Ok(format!(
                "{parameters} => {{\n{}\n{}}}",
                nested.finish(),
                style.indentation().repeat(depth)
            ))
        }
    }
}
