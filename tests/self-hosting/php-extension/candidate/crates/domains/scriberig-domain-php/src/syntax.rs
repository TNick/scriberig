//! Serializes PHP grammar nodes with exact source ranges and relationships.

use serde::{Deserialize, Serialize};
use tree_sitter::Node;

/// One syntax node in the complete ordered PHP syntax tree.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhpNode {
    /// Grammar node kind.
    pub kind: String,
    /// Whether the grammar marks this node as named.
    pub named: bool,
    /// Whether the parser synthesized this node.
    pub missing: bool,
    /// Whether this node is a parser error node.
    pub error: bool,
    /// Inclusive source byte offset.
    pub start_byte: usize,
    /// Exclusive source byte offset.
    pub end_byte: usize,
    /// Zero-based start row.
    pub start_row: usize,
    /// Zero-based start byte column.
    pub start_column: usize,
    /// Zero-based end row.
    pub end_row: usize,
    /// Zero-based end byte column.
    pub end_column: usize,
    /// Parent field relation when the grammar exposes one.
    pub field: Option<String>,
    /// Exact leaf text when this node has no children.
    pub text: Option<String>,
    /// Child nodes in grammar source order.
    pub children: Vec<PhpNode>,
}

/// Flat syntax record with deterministic structural path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhpNodeRecord {
    /// Stable structural path from root.
    pub path: String,
    /// Grammar node kind.
    pub kind: String,
    /// Whether the grammar marks this node as named.
    pub named: bool,
    /// Whether this node is missing.
    pub missing: bool,
    /// Whether this node is an error node.
    pub error: bool,
    /// Inclusive source byte offset.
    pub start_byte: usize,
    /// Exclusive source byte offset.
    pub end_byte: usize,
    /// Zero-based start row.
    pub start_row: usize,
    /// Zero-based start byte column.
    pub start_column: usize,
    /// Zero-based end row.
    pub end_row: usize,
    /// Zero-based end byte column.
    pub end_column: usize,
    /// Parent field relation when available.
    pub field: Option<String>,
    /// Exact leaf text when appropriate.
    pub text: Option<String>,
}

/// Builds the serializable tree while preserving every grammar child.
///
/// # Parameters
/// - `node`: Tree-sitter syntax node.
/// - `source`: Exact source text.
/// - `field`: Parent field relation.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// A recursively ordered serializable node.
pub(crate) fn build_node(node: Node<'_>, source: &str, field: Option<String>) -> PhpNode {
    let mut cursor = node.walk();
    let mut children = Vec::new();
    if cursor.goto_first_child() {
        loop {
            children.push(build_node(
                cursor.node(),
                source,
                cursor.field_name().map(str::to_owned),
            ));
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    let range = node.byte_range();
    PhpNode {
        kind: node.kind().to_owned(),
        named: node.is_named(),
        missing: node.is_missing(),
        error: node.is_error(),
        start_byte: range.start,
        end_byte: range.end,
        start_row: node.start_position().row,
        start_column: node.start_position().column,
        end_row: node.end_position().row,
        end_column: node.end_position().column,
        field,
        text: (node.child_count() == 0).then(|| source[range].to_owned()),
        children,
    }
}

/// Flattens a syntax tree in deterministic pre-order.
///
/// # Parameters
/// - `node`: Current tree node.
/// - `path`: Structural path to the node.
/// - `records`: Destination record collection.
/// - `crate`: Internal visibility marker.
pub(crate) fn flatten_node(node: &PhpNode, path: &str, records: &mut Vec<PhpNodeRecord>) {
    records.push(PhpNodeRecord {
        path: path.to_owned(),
        kind: node.kind.clone(),
        named: node.named,
        missing: node.missing,
        error: node.error,
        start_byte: node.start_byte,
        end_byte: node.end_byte,
        start_row: node.start_row,
        start_column: node.start_column,
        end_row: node.end_row,
        end_column: node.end_column,
        field: node.field.clone(),
        text: node.text.clone(),
    });
    for (index, child) in node.children.iter().enumerate() {
        flatten_node(child, &format!("{path}/{index}"), records);
    }
}

/// Collects parser errors and missing nodes with exact start positions.
///
/// # Parameters
/// - `node`: Current parser node.
/// - `diagnostics`: Destination ordered messages.
/// - `crate`: Internal visibility marker.
pub(crate) fn collect_errors(node: Node<'_>, diagnostics: &mut Vec<String>) {
    if node.is_error() || node.is_missing() {
        let point = node.start_position();
        let category = if node.is_missing() {
            "missing syntax"
        } else {
            "syntax error"
        };
        diagnostics.push(format!(
            "{category} at {}:{} (bytes {}..{})",
            point.row + 1,
            point.column + 1,
            node.start_byte(),
            node.end_byte()
        ));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors(child, diagnostics);
    }
}
