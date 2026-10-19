//! Extracts common named declarations from the complete PHP syntax tree.

use serde::{Deserialize, Serialize};
use tree_sitter::Node;

/// Named PHP declaration with a stable source-local identity key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhpDeclaration {
    /// Shared code-domain kind such as `php.class` or `php.method`.
    pub kind: String,
    /// Human-readable declaration name.
    pub name: String,
    /// Stable key unique within this source document.
    pub key: String,
    /// Inclusive declaration source byte offset.
    pub start_byte: usize,
    /// Exclusive declaration source byte offset.
    pub end_byte: usize,
}

/// Extracts supported named declarations without dropping syntax nodes.
///
/// - `root`: Complete Tree-sitter PHP syntax root.
/// - `source`: Exact source bytes for declaration names.
/// - `crate`: Internal visibility marker.
///
/// Declarations ordered by source range.
/// # Returns
/// The operation result.
pub(crate) fn extract(root: Node<'_>, source: &str) -> Vec<PhpDeclaration> {
    let mut declarations = Vec::new();
    visit(root, source, "", &mut declarations);
    declarations.sort_by_key(|declaration| (declaration.start_byte, declaration.end_byte));
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    for declaration in &mut declarations {
        let count = seen.entry(declaration.key.clone()).or_default();
        *count += 1;
        if *count > 1 {
            declaration.key = format!("{}#{}", declaration.key, count);
        }
    }
    declarations
}

/// Visits grammar nodes and extracts declarations with a `name` field.
///
/// - `node`: Current parser node.
/// - `source`: Exact source bytes for declaration names.
/// - `scope`: Parent declaration key.
/// - `declarations`: Destination collection.
/// - `crate`: Internal visibility marker.
fn visit(node: Node<'_>, source: &str, scope: &str, declarations: &mut Vec<PhpDeclaration>) {
    let mut child_scope = scope.to_owned();
    if let Some(kind) = declaration_kind(node.kind())
        && let Some(name_node) = node.child_by_field_name("name")
    {
        let name = name_node.utf8_text(source.as_bytes()).unwrap_or_default();
        let key = if scope.is_empty() {
            name.to_owned()
        } else {
            format!("{scope}::{name}")
        };
        declarations.push(PhpDeclaration {
            kind: kind.to_owned(),
            name: name.to_owned(),
            key: key.clone(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        });
        child_scope = key;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, source, &child_scope, declarations);
    }
}

/// Maps supported grammar node kinds to shared PHP code-domain kinds.
///
/// - `kind`: Tree-sitter grammar node kind.
///
/// The public semantic kind, or none for non-declarations.
/// # Returns
/// The operation result.
fn declaration_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "namespace_definition" => Some("php.namespace"),
        "class_declaration" => Some("php.class"),
        "interface_declaration" => Some("php.interface"),
        "trait_declaration" => Some("php.trait"),
        "enum_declaration" => Some("php.enum"),
        "function_definition" => Some("php.function"),
        "method_declaration" => Some("php.method"),
        _ => None,
    }
}
