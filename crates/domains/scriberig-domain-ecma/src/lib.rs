//! Parses JavaScript and TypeScript into serializable syntax-tree records.

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast_visit::utf8_to_utf16::Utf8ToUtf16;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A parsed JavaScript or TypeScript document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EcmaDocument {
    /// Adapter identifier selected from the source path.
    pub adapter: String,
    /// Complete ESTree-compatible syntax tree.
    pub tree: Value,
    /// Every syntax object in deterministic traversal order.
    pub nodes: Vec<EcmaNode>,
}

/// One serializable syntax-tree node.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EcmaNode {
    /// JSON pointer from the document tree to this node.
    pub path: String,
    /// `ESTree` node type.
    pub kind: String,
    /// Optional UTF-16 source start offset.
    pub start: Option<u64>,
    /// Optional UTF-16 source end offset.
    pub end: Option<u64>,
    /// Complete node payload.
    pub value: Map<String, Value>,
}

/// A parser failure with source-independent diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcmaError {
    /// Whether the parser stopped before producing a valid AST.
    pub fatal: bool,
    /// Parser diagnostics in source order.
    pub diagnostics: Vec<String>,
}

impl std::fmt::Display for EcmaError {
    /// Formats the first parser diagnostic for human-facing callers.
    ///
    /// - `self`: Parser failure to format.
    /// - `formatter`: Formatting destination.
    ///
    /// A formatting result.
    ///
    /// Returns a formatting error when the destination rejects output.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.fatal {
            formatter.write_str("fatal ECMAScript syntax error")?;
        } else {
            formatter.write_str("ECMAScript syntax error")?;
        }
        if let Some(message) = self.diagnostics.first() {
            write!(formatter, ": {message}")?;
        }
        Ok(())
    }
}

impl std::error::Error for EcmaError {}

/// Parses JavaScript, JSX, TypeScript, or TSX based on the file path.
///
/// - `path`: Source path used to select the ECMAScript language mode.
/// - `source`: UTF-8 source text.
///
/// A complete serializable syntax tree and its individual nodes.
///
/// Returns all parser diagnostics when the source is incomplete or invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn parse(path: &Path, source: &str) -> Result<EcmaDocument, EcmaError> {
    let source_type = SourceType::from_path(path).map_err(|error| EcmaError {
        fatal: true,
        diagnostics: vec![format!("unsupported ECMAScript path: {error}")],
    })?;
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, source_type).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        return Err(EcmaError {
            fatal: parsed.fatal_error,
            diagnostics: parsed.diagnostics.iter().map(ToString::to_string).collect(),
        });
    }
    Utf8ToUtf16::new(source).convert_program(&mut parsed.program);
    let tree = serde_json::from_str(
        &parsed
            .program
            .to_pretty_estree_json(!source_type.is_javascript(), false),
    )
    .map_err(|error| EcmaError {
        fatal: true,
        diagnostics: vec![format!("failed to encode syntax tree: {error}")],
    })?;
    let mut nodes = Vec::new();
    collect_nodes(&tree, "", &mut nodes);
    Ok(EcmaDocument {
        adapter: if source_type.is_javascript() {
            "javascript".to_owned()
        } else {
            "typescript".to_owned()
        },
        tree,
        nodes,
    })
}

/// Collects every `ESTree` object with a type field.
///
/// - `value`: Current JSON value.
/// - `path`: JSON pointer of the current value.
/// - `nodes`: Destination node list.
///
/// Unit after deterministic depth-first traversal.
fn collect_nodes(value: &Value, path: &str, nodes: &mut Vec<EcmaNode>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(kind)) = object.get("type") {
                nodes.push(EcmaNode {
                    path: path.to_owned(),
                    kind: kind.clone(),
                    start: object.get("start").and_then(Value::as_u64),
                    end: object.get("end").and_then(Value::as_u64),
                    value: object.clone(),
                });
            }
            for (key, child) in object {
                collect_nodes(child, &format!("{path}/{key}"), nodes);
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                collect_nodes(child, &format!("{path}/{index}"), nodes);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses JavaScript and exposes a complete navigable syntax tree.
    #[test]
    fn parses_javascript_nodes() {
        let document = parse(
            Path::new("main.js"),
            "export function answer() { return 42; }",
        )
        .expect("JavaScript parses");
        assert_eq!(document.adapter, "javascript");
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.kind == "FunctionDeclaration")
        );
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.kind == "ReturnStatement")
        );
    }

    /// Rejects syntax errors so callers can request source repair and retry.
    #[test]
    fn reports_syntax_errors() {
        let error =
            parse(Path::new("broken.ts"), "const value: = 1;").expect_err("invalid TypeScript");
        assert_ne!(error.diagnostics, Vec::<String>::new());
    }
}
