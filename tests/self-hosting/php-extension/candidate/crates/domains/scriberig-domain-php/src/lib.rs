//! Parses PHP into complete deterministic syntax and declaration records.

mod declarations;
mod syntax;

use std::fmt::{Display, Formatter};
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use declarations::PhpDeclaration;
pub use syntax::{PhpNode, PhpNodeRecord};

/// Versioned parsed PHP document retaining its complete syntax tree.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhpDocument {
    /// Stable source adapter identity.
    pub adapter: String,
    /// Adapter schema version.
    pub schema_version: u32,
    /// Complete syntax tree, including unnamed nodes.
    pub tree: PhpNode,
    /// Deterministic depth-first syntax records.
    pub nodes: Vec<PhpNodeRecord>,
    /// Named declarations useful for shared semantic navigation.
    pub declarations: Vec<PhpDeclaration>,
}

/// Structured PHP parser diagnostics with source positions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhpError {
    /// Diagnostics in deterministic source traversal order.
    pub diagnostics: Vec<String>,
}

impl Display for PhpError {
    /// Formats a PHP parser failure.
    ///
    /// - `self`: Parser error to format.
    /// - `formatter`: Formatting destination.
    ///
    /// A formatting result.
    ///
    /// Returns an error when the formatter rejects output.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PHP syntax error")?;
        if let Some(diagnostic) = self.diagnostics.first() {
            write!(formatter, ": {diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PhpError {}

/// Parses PHP source inertly into deterministic syntax and declaration data.
///
/// - `path`: Project-relative source path used for adapter context.
/// - `source`: Exact UTF-8 source text.
///
/// A versioned PHP document with the complete syntax tree.
///
/// Returns parser initialization, missing-tree, error-node, or missing-node diagnostics.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn parse(path: &Path, source: &str) -> Result<PhpDocument, PhpError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
        .map_err(|error| PhpError {
            diagnostics: vec![format!("failed to initialize PHP parser: {error}")],
        })?;
    let parsed = parser.parse(source, None).ok_or_else(|| PhpError {
        diagnostics: vec!["PHP parser returned no syntax tree".to_owned()],
    })?;
    let mut diagnostics = Vec::new();
    syntax::collect_errors(parsed.root_node(), &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(PhpError { diagnostics });
    }
    let tree = syntax::build_node(parsed.root_node(), source, None);
    let mut nodes = Vec::new();
    syntax::flatten_node(&tree, "", &mut nodes);
    let declarations = declarations::extract(parsed.root_node(), source);
    Ok(PhpDocument {
        adapter: adapter_for(path).to_owned(),
        schema_version: 1,
        tree,
        nodes,
        declarations,
    })
}

/// Returns the stable adapter identifier for a PHP source path.
///
/// - `path`: Project-relative PHP path.
///
/// The versioned PHP adapter identifier.
/// # Returns
/// The operation result.
fn adapter_for(path: &Path) -> &'static str {
    let _ = path;
    "php"
}
