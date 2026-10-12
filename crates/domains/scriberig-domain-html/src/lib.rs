//! Parses HTML documents into safe, meaningful semantic nodes.

use std::io;

use kuchiki::traits::TendrilSink;
use kuchiki::{NodeData, NodeRef};
use serde::{Deserialize, Serialize};

/// A parsed HTML document with its meaningful semantic descendants.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HtmlDocument {
    /// Document title when one is present.
    pub title: Option<String>,
    /// Meaningful nodes in document order.
    pub nodes: Vec<HtmlNode>,
    /// Normalized HTML serialization of the parsed document.
    pub rendered: Vec<u8>,
}

/// A meaningful or opaque HTML node.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HtmlNode {
    /// Element name or `text`/`opaque` node kind.
    pub kind: String,
    /// Visible or source text associated with the node.
    pub text: String,
    /// HTML attributes on the element.
    pub attributes: Vec<(String, String)>,
    /// Nesting depth in the parsed document.
    pub depth: usize,
}

impl HtmlDocument {
    /// Encodes the semantic document as stable versioned JSON bytes.
    ///
    /// # Returns
    /// Canonical JSON representation of the parsed document.
    ///
    /// # Errors
    /// Returns a serialization error when the document cannot be encoded.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

impl HtmlNode {
    /// Encodes one semantic node as stable versioned JSON bytes.
    ///
    /// # Returns
    /// Canonical JSON representation of the node.
    ///
    /// # Errors
    /// Returns a serialization error when the node cannot be encoded.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

/// Parses an HTML byte stream without executing or fetching anything.
///
/// # Parameters
/// - `bytes`: HTML source bytes.
///
/// # Returns
/// Parsed semantic nodes and normalized HTML bytes.
///
/// # Errors
/// Returns an error when the source cannot be serialized after parsing.
pub fn parse(bytes: &[u8]) -> Result<HtmlDocument, io::Error> {
    let document = kuchiki::parse_html().from_utf8().one(bytes);
    let title = document
        .select_first("title")
        .ok()
        .map(|node| node.text_contents().trim().to_owned())
        .filter(|value| !value.is_empty());
    let mut nodes = Vec::new();
    collect_nodes(&document, 0, &mut nodes);
    let mut rendered = Vec::new();
    document.serialize(&mut rendered)?;
    Ok(HtmlDocument {
        title,
        nodes,
        rendered,
    })
}

/// Collects meaningful descendants while retaining active content as opaque.
///
/// # Parameters
/// - `node`: Current DOM node.
/// - `depth`: Current nesting depth.
/// - `nodes`: Destination semantic node list.
///
/// # Returns
/// Unit after descendants have been collected.
fn collect_nodes(node: &NodeRef, depth: usize, nodes: &mut Vec<HtmlNode>) {
    for child in node.children() {
        let (kind, text, attributes) = match child.data() {
            NodeData::Element(element) => {
                let name = element.name.local.to_string();
                let kind = if is_meaningful(&name) {
                    name
                } else {
                    "opaque".to_owned()
                };
                let attributes = element
                    .attributes
                    .borrow()
                    .map
                    .iter()
                    .map(|(name, value)| (name.local.to_string(), value.value.clone()))
                    .collect();
                (kind, child.text_contents().trim().to_owned(), attributes)
            }
            NodeData::Text(contents) => {
                let text = contents.borrow().trim().to_owned();
                if text.is_empty() {
                    continue;
                }
                ("text".to_owned(), text, Vec::new())
            }
            NodeData::Comment(contents) => {
                ("opaque".to_owned(), contents.borrow().clone(), Vec::new())
            }
            _ => continue,
        };
        nodes.push(HtmlNode {
            kind,
            text,
            attributes,
            depth,
        });
        collect_nodes(&child, depth + 1, nodes);
    }
}

/// Identifies elements useful for progressive semantic disclosure.
///
/// # Parameters
/// - `name`: Lowercase HTML element name.
///
/// # Returns
/// Whether the element receives a semantic node kind.
fn is_meaningful(name: &str) -> bool {
    matches!(
        name,
        "a" | "article"
            | "blockquote"
            | "code"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "img"
            | "li"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "td"
            | "th"
            | "tr"
            | "ul"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extracts meaningful structure without executing active content.
    #[test]
    fn parses_meaningful_nodes_and_preserves_script_source() {
        let document = parse(
            br"<html><head><title>Guide</title></head><body><h1>Title</h1><p>Hello</p><script>bad()</script></body></html>",
        )
        .expect("HTML parses");
        assert_eq!(document.title.as_deref(), Some("Guide"));
        assert!(document.nodes.iter().any(|node| node.kind == "h1"));
        assert!(document.nodes.iter().any(|node| node.kind == "opaque"));
        assert!(String::from_utf8_lossy(&document.rendered).contains("bad()"));
    }
}
