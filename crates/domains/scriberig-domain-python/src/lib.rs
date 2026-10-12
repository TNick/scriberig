//! Parses Python source into a complete, serializable syntax tree.
//!
//! Canonical Python state is stored through Scriberig's shared universal model;
//! this crate does not create an independent source of truth.

use std::fmt::{Display, Formatter};
use std::path::Path;

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

/// A parsed Python source document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PythonDocument {
    /// Adapter identifier for the parsed source.
    pub adapter: String,
    /// Complete syntax tree rooted at the module node.
    pub tree: PythonNode,
    /// Every syntax node in deterministic depth-first order.
    pub nodes: Vec<PythonNodeRecord>,
}

/// One serializable Python syntax node.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PythonNodeRecord {
    /// Stable path from the root node to this node.
    pub path: String,
    /// Grammar node kind.
    pub kind: String,
    /// Whether this is a named grammar node.
    pub named: bool,
    /// Whether the parser synthesized this node.
    pub missing: bool,
    /// Inclusive UTF-8 byte start offset.
    pub start_byte: usize,
    /// Exclusive UTF-8 byte end offset.
    pub end_byte: usize,
    /// Zero-based start line.
    pub start_line: usize,
    /// Zero-based start column in bytes.
    pub start_column: usize,
    /// Zero-based end line.
    pub end_line: usize,
    /// Zero-based end column in bytes.
    pub end_column: usize,
    /// Grammar field containing this node, when present.
    pub field: Option<String>,
    /// Source text for leaf nodes.
    pub text: Option<String>,
}

impl PythonDocument {
    /// Returns the adapter identifier for this parsed document.
    ///
    /// # Returns
    /// The Python adapter identifier.
    #[must_use]
    pub fn adapter(&self) -> &str {
        &self.adapter
    }
}

/// A serializable syntax tree node with ordered children.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PythonNode {
    /// Grammar node kind.
    pub kind: String,
    /// Whether this is a named grammar node.
    pub named: bool,
    /// Whether the parser synthesized this node.
    pub missing: bool,
    /// Inclusive UTF-8 byte start offset.
    pub start_byte: usize,
    /// Exclusive UTF-8 byte end offset.
    pub end_byte: usize,
    /// Zero-based start line.
    pub start_line: usize,
    /// Zero-based start column in bytes.
    pub start_column: usize,
    /// Zero-based end line.
    pub end_line: usize,
    /// Zero-based end column in bytes.
    pub end_column: usize,
    /// Grammar field containing this node, when present.
    pub field: Option<String>,
    /// Source text for leaf nodes.
    pub text: Option<String>,
    /// Ordered child nodes.
    pub children: Vec<PythonNode>,
}

/// A Python parser failure with source-location diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PythonError {
    /// Parser diagnostics in source order.
    pub diagnostics: Vec<String>,
}

impl Display for PythonError {
    /// Formats the first Python parser diagnostic.
    ///
    /// # Parameters
    /// - `self`: Parser failure to format.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Python syntax error")?;
        if let Some(message) = self.diagnostics.first() {
            write!(formatter, ": {message}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PythonError {}

/// Parses Python source into a complete syntax tree.
///
/// # Parameters
/// - `path`: Source path used for adapter context.
/// - `source`: UTF-8 Python source text.
///
/// # Returns
/// A complete serializable syntax tree and its individual nodes.
///
/// # Errors
/// Returns all syntax-error diagnostics when the source is incomplete or invalid.
pub fn parse(path: &Path, source: &str) -> Result<PythonDocument, PythonError> {
    let mut parser = Parser::new();
    let language = tree_sitter_python::LANGUAGE.into();
    parser
        .set_language(&language)
        .map_err(|error| PythonError {
            diagnostics: vec![format!("failed to initialize Python parser: {error}")],
        })?;
    let tree = parser.parse(source, None).ok_or_else(|| PythonError {
        diagnostics: vec!["Python parser returned no syntax tree".to_owned()],
    })?;
    let mut diagnostics = Vec::new();
    collect_errors(tree.root_node(), source, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(PythonError { diagnostics });
    }
    let root = build_node(tree.root_node(), source, None);
    let mut nodes = Vec::new();
    flatten_node(&root, "", &mut nodes);
    Ok(PythonDocument {
        adapter: adapter_for(path).to_owned(),
        tree: root,
        nodes,
    })
}

/// Selects the public adapter identifier for a Python source path.
///
/// # Parameters
/// - `path`: Python source path.
///
/// # Returns
/// The Python adapter identifier.
fn adapter_for(path: &Path) -> &'static str {
    let _ = path;
    "python"
}

/// Builds the serializable tree representation for one parser node.
///
/// # Parameters
/// - `node`: Tree-sitter node to serialize.
/// - `source`: Original source text.
/// - `field`: Optional parent grammar field name.
///
/// # Returns
/// A serializable node with recursively ordered children.
fn build_node(node: Node<'_>, source: &str, field: Option<String>) -> PythonNode {
    let mut cursor = node.walk();
    let mut children = Vec::new();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            let child_field = cursor.field_name().map(str::to_owned);
            children.push(build_node(child, source, child_field));
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    let text = if node.child_count() == 0 {
        Some(source[node.byte_range()].to_owned())
    } else {
        None
    };
    PythonNode {
        kind: node.kind().to_owned(),
        named: node.is_named(),
        missing: node.is_missing(),
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: node.start_position().row,
        start_column: node.start_position().column,
        end_line: node.end_position().row,
        end_column: node.end_position().column,
        field,
        text,
        children,
    }
}

/// Collects every syntax node in depth-first order.
///
/// # Parameters
/// - `node`: Current serialized node.
/// - `path`: Stable path to the node.
/// - `nodes`: Destination flat node list.
///
/// # Returns
/// Unit after the subtree is flattened.
fn flatten_node(node: &PythonNode, path: &str, nodes: &mut Vec<PythonNodeRecord>) {
    nodes.push(PythonNodeRecord {
        path: path.to_owned(),
        kind: node.kind.clone(),
        named: node.named,
        missing: node.missing,
        start_byte: node.start_byte,
        end_byte: node.end_byte,
        start_line: node.start_line,
        start_column: node.start_column,
        end_line: node.end_line,
        end_column: node.end_column,
        field: node.field.clone(),
        text: node.text.clone(),
    });
    for (index, child) in node.children.iter().enumerate() {
        flatten_node(child, &format!("{path}/{index}"), nodes);
    }
}

/// Collects parser error nodes with source coordinates.
///
/// # Parameters
/// - `node`: Current parser node.
/// - `source`: Original source text.
/// - `diagnostics`: Destination diagnostic messages.
///
/// # Returns
/// Unit after the subtree has been inspected.
fn collect_errors(node: Node<'_>, source: &str, diagnostics: &mut Vec<String>) {
    if node.is_error() || node.is_missing() {
        let point = node.start_position();
        let label = if node.is_missing() {
            "missing syntax"
        } else {
            "syntax error"
        };
        let excerpt = source
            .get(node.byte_range())
            .filter(|text| !text.is_empty())
            .unwrap_or("unexpected input");
        diagnostics.push(format!(
            "{label} at {}:{}: {excerpt}",
            point.row + 1,
            point.column + 1
        ));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors(child, source, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses Python declarations and preserves nested syntax nodes.
    #[test]
    fn parses_python_nodes() {
        let source = "# greeting\ndef greet(name: str) -> str:\n    return f'Hi {name}'\n";
        let document = parse(Path::new("greet.py"), source).expect("Python parses");
        assert_eq!(document.adapter, "python");
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.kind == "function_definition")
        );
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.kind == "return_statement")
        );
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.text.as_deref() == Some("# greeting"))
        );
        assert_eq!(document.tree.start_byte, 0);
    }

    /// Reports syntax errors with source coordinates for recovery callers.
    #[test]
    fn reports_syntax_errors() {
        let error = parse(Path::new("broken.py"), "def broken(:\n").expect_err("invalid Python");
        assert!(
            error
                .diagnostics
                .iter()
                .any(|message| message.contains("syntax"))
        );
        assert!(
            error
                .diagnostics
                .iter()
                .any(|message| message.contains("1:"))
        );
    }

    /// Preserves deterministic child ordering and source ranges.
    #[test]
    fn preserves_ranges_and_order() {
        let document = parse(Path::new("main.py"), "x = 1\ny = x + 2\n").expect("Python parses");
        let assignments = document
            .nodes
            .iter()
            .filter(|node| node.kind == "assignment")
            .collect::<Vec<_>>();
        assert_eq!(assignments.len(), 2);
        assert!(assignments[0].start_byte < assignments[1].start_byte);
        assert_eq!(assignments[0].text, None);
    }
}
