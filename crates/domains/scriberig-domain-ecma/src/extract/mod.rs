//! Builds the semantic source model from the `oxc` syntax tree.
//!
//! The entry point is [`parse_source`], which reuses the crate's `oxc` parser
//! front end and then models only the constructs documented in
//! [`crate::source`]. Every other node is refused with a structured
//! [`crate::EcmaError`] naming the construct and its byte offset, so a caller
//! never receives an approximated or partially modeled document.

mod declarations;
mod expressions;
mod kinds;
mod statements;

use std::path::Path;

use oxc_span::Span;

use crate::EcmaError;
use crate::source::EcmaSource;

/// Parses a document into the semantic source model of the supported subset.
///
/// - `path`: Source path used to select the ECMAScript language mode.
/// - `source`: UTF-8 source text.
///
/// The semantic model rather than the syntax tree.
///
/// # Returns
/// The modeled document when every node is in the supported subset.
///
/// # Errors
/// Returns a structured error when the source has a syntax error or contains a
/// construct outside the supported subset.
pub fn parse_source(path: &Path, source: &str) -> Result<EcmaSource, EcmaError> {
    crate::with_program(path, source, |program| {
        if let Some(comment) = program.comments.first() {
            return Err(unsupported(
                "comment trivia (not preserved by the semantic model)",
                comment.span,
            ));
        }
        if let Some(directive) = program.directives.first() {
            return Err(unsupported("directive", directive.span));
        }
        Ok(EcmaSource {
            items: declarations::items(&program.body)?,
        })
    })
}

/// Builds the structured refusal for one unsupported node.
///
/// - `kind`: Construct name reported to the caller.
/// - `span`: Source span of the rejected node.
///
/// # Returns
/// A fatal parser error naming the construct and its byte offset.
pub(crate) fn unsupported(kind: &str, span: Span) -> EcmaError {
    EcmaError {
        fatal: true,
        diagnostics: vec![format!(
            "unsupported ECMAScript construct: {kind} at offset {}",
            span.start
        )],
    }
}
