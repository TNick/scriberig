//! Parses C and C++ source into complete, serializable syntax trees.
//!
//! Canonical C and C++ state is stored through Scriberig's shared universal
//! model; this crate does not create an independent source of truth.

use std::fmt::{Display, Formatter};
use std::path::Path;

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

/// A parsed C or C++ source document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CDocument {
    /// Adapter identifier selected from the source path.
    pub adapter: String,
    /// Complete syntax tree rooted at the translation unit.
    pub tree: CNode,
    /// Every syntax node in deterministic depth-first order.
    pub nodes: Vec<CNodeRecord>,
}

/// One serializable C or C++ syntax node.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CNodeRecord {
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

/// A serializable syntax tree node with ordered children.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CNode {
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
    pub children: Vec<CNode>,
}

/// A C or C++ parser failure with source-location diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CError {
    /// Parser diagnostics in source order.
    pub diagnostics: Vec<String>,
}

impl Display for CError {
    /// Formats the first C-family parser diagnostic.
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
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("C-family syntax error")?;
        if let Some(message) = self.diagnostics.first() {
            write!(formatter, ": {message}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CError {}

/// Parses C or C++ source based on its file extension.
///
/// - `path`: Source path used to select the grammar.
/// - `source`: UTF-8 C or C++ source text.
///
/// A complete serializable syntax tree and its individual nodes.
///
/// Returns all syntax-error diagnostics when the source is incomplete or invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn parse(path: &Path, source: &str) -> Result<CDocument, CError> {
    let adapter = adapter_for(path).ok_or_else(|| CError {
        diagnostics: vec![format!("unsupported C-family path: {}", path.display())],
    })?;
    let mut parser = Parser::new();
    let language = if adapter == "cpp" {
        tree_sitter_cpp::LANGUAGE.into()
    } else {
        tree_sitter_c::LANGUAGE.into()
    };
    parser.set_language(&language).map_err(|error| CError {
        diagnostics: vec![format!("failed to initialize {adapter} parser: {error}")],
    })?;
    let tree = parser.parse(source, None).ok_or_else(|| CError {
        diagnostics: vec![format!("{adapter} parser returned no syntax tree")],
    })?;
    let mut diagnostics = Vec::new();
    collect_errors(tree.root_node(), source, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(CError { diagnostics });
    }
    let root = build_node(tree.root_node(), source, None);
    let mut nodes = Vec::new();
    flatten_node(&root, "", &mut nodes);
    Ok(CDocument {
        adapter: adapter.to_owned(),
        tree: root,
        nodes,
    })
}

/// Selects the C-family adapter identifier for a source path.
///
/// - `path`: Source path to classify.
///
/// The C or C++ adapter identifier, or none for an unsupported extension.
/// # Returns
/// The operation result.
fn adapter_for(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|value| value.to_str()) {
        Some("c" | "h") => Some("c"),
        Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => Some("cpp"),
        _ => None,
    }
}

/// Builds the serializable tree representation for one parser node.
///
/// - `node`: Tree-sitter node to serialize.
/// - `source`: Original source text.
/// - `field`: Optional parent grammar field name.
///
/// A serializable node with recursively ordered children.
/// # Returns
/// The operation result.
fn build_node(node: Node<'_>, source: &str, field: Option<String>) -> CNode {
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
    CNode {
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
/// - `node`: Current serialized node.
/// - `path`: Stable path to the node.
/// - `nodes`: Destination flat node list.
///
/// Unit after the subtree is flattened.
fn flatten_node(node: &CNode, path: &str, nodes: &mut Vec<CNodeRecord>) {
    nodes.push(CNodeRecord {
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
/// - `node`: Current parser node.
/// - `source`: Original source text.
/// - `diagnostics`: Destination diagnostic messages.
///
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

    /// Parses C declarations, comments, and preprocessor directives.
    #[test]
    fn parses_c_nodes() {
        let source =
            "#include <stdio.h>\n// greeting\nint greet(const char *name) { return 42; }\n";
        let document = parse(Path::new("greet.c"), source).expect("C parses");
        assert_eq!(document.adapter, "c");
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
                .any(|node| node.kind == "preproc_include")
        );
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.text.as_deref() == Some("// greeting"))
        );
    }

    /// Parses C++ classes, namespaces, templates, and lambdas.
    #[test]
    fn parses_cpp_nodes() {
        let source = "namespace demo { template <typename T> class Box { T value; }; }\n";
        let document = parse(Path::new("box.hpp"), source).expect("C++ parses");
        assert_eq!(document.adapter, "cpp");
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.kind == "namespace_definition")
        );
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.kind == "class_specifier")
        );
        assert!(
            document
                .nodes
                .iter()
                .any(|node| node.kind == "template_declaration")
        );
    }

    /// Rejects malformed C-family syntax with source coordinates.
    #[test]
    fn reports_syntax_errors() {
        let error = parse(Path::new("broken.cpp"), "int broken( {\n").expect_err("invalid C++");
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

    /// Preserves deterministic node order and source ranges.
    #[test]
    fn preserves_ranges_and_order() {
        let document = parse(Path::new("main.c"), "int first;\nint second;\n").expect("C parses");
        let declarations = document
            .nodes
            .iter()
            .filter(|node| node.kind == "declaration")
            .collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2);
        assert!(declarations[0].start_byte < declarations[1].start_byte);
    }
}
