//! Parses common structured project data into deterministic value nodes.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A parsed JSON, YAML, or TOML document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DataDocument {
    /// Stable format identifier selected from the source path.
    pub format: String,
    /// Complete normalized value tree.
    pub value: Value,
    /// Every object and array value in deterministic traversal order.
    pub nodes: Vec<DataNode>,
}

/// One addressable value in a structured document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DataNode {
    /// JSON Pointer from the root value to this node.
    pub path: String,
    /// Value kind such as `object`, `array`, or `string`.
    pub kind: String,
    /// Node value.
    pub value: Value,
}

/// Reports structured-data parsing failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataError {
    /// Human-readable parser diagnostics.
    pub diagnostics: Vec<String>,
}

impl std::fmt::Display for DataError {
    /// Formats the first structured-data diagnostic.
    ///
    /// # Parameters
    /// - `self`: Structured-data error.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            self.diagnostics
                .first()
                .map_or("structured data parse failed", String::as_str),
        )
    }
}

impl std::error::Error for DataError {}

/// Parses JSON, JSONC, YAML, or TOML according to the file extension.
///
/// # Parameters
/// - `path`: Source path used to select the parser.
/// - `source`: UTF-8 source text.
///
/// # Returns
/// A normalized value tree and deterministic node list.
///
/// # Errors
/// Returns a structured diagnostic for unsupported paths or invalid input.
pub fn parse(path: &Path, source: &str) -> Result<DataDocument, DataError> {
    let extension = path.extension().and_then(|value| value.to_str());
    let (format, value) = match extension {
        Some("json") => (
            "json",
            serde_json::from_str(source).map_err(|error| error.to_string()),
        ),
        Some("jsonc") => (
            "jsonc",
            serde_json::from_str(&strip_json_comments(source)).map_err(|error| error.to_string()),
        ),
        Some("yaml" | "yml") => (
            "yaml",
            serde_yaml::from_str::<serde_yaml::Value>(source)
                .map_err(|error| error.to_string())
                .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string())),
        ),
        Some("toml") => (
            "toml",
            toml::from_str::<toml::Value>(source)
                .map_err(|error| error.to_string())
                .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string())),
        ),
        _ => {
            return Err(DataError {
                diagnostics: vec![format!(
                    "unsupported structured-data path: {}",
                    path.display()
                )],
            });
        }
    };
    let value = value.map_err(|error| DataError {
        diagnostics: vec![error.clone()],
    })?;
    let mut nodes = Vec::new();
    collect_nodes(&value, "", &mut nodes);
    Ok(DataDocument {
        format: format.to_owned(),
        value,
        nodes,
    })
}

/// Removes JSONC comments while preserving strings and newlines.
///
/// # Parameters
/// - `source`: JSONC source text.
///
/// # Returns
/// JSON text with line and block comments replaced by whitespace.
fn strip_json_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut string = false;
    while let Some(character) = chars.next() {
        if character == '"' {
            output.push(character);
            string = !string;
            continue;
        }
        if string {
            output.push(character);
            if character == '\\' {
                if let Some(escaped) = chars.next() {
                    output.push(escaped);
                }
            }
            continue;
        }
        if character == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
            continue;
        }
        if character == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(next) = chars.next() {
                if next == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
                if next == '\n' {
                    output.push('\n');
                }
            }
            continue;
        }
        output.push(character);
    }
    output
}

/// Collects every value node in deterministic depth-first order.
///
/// # Parameters
/// - `value`: Current value.
/// - `path`: JSON Pointer of the current value.
/// - `nodes`: Destination node list.
fn collect_nodes(value: &Value, path: &str, nodes: &mut Vec<DataNode>) {
    nodes.push(DataNode {
        path: path.to_owned(),
        kind: value_kind(value).to_owned(),
        value: value.clone(),
    });
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                collect_nodes(child, &format!("{path}/{}", escape_pointer(key)), nodes);
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

/// Returns the stable kind label for one JSON value.
///
/// # Parameters
/// - `value`: Value to classify.
///
/// # Returns
/// Stable value kind label.
fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Escapes a key for JSON Pointer syntax.
///
/// # Parameters
/// - `key`: Object key.
///
/// # Returns
/// Escaped pointer segment.
fn escape_pointer(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses JSONC comments and preserves deterministic child paths.
    #[test]
    fn parses_jsonc_nodes() {
        let document = parse(
            Path::new("settings.jsonc"),
            "{\"name\": \"demo\", // comment\n \"items\": [1]}\n",
        )
        .expect("JSONC parses");
        assert_eq!(document.format, "jsonc");
        assert!(document.nodes.iter().any(|node| node.path == "/items/0"));
    }

    /// Parses YAML and TOML using the same normalized node model.
    #[test]
    fn parses_yaml_and_toml() {
        let yaml = parse(Path::new("config.yaml"), "name: demo\n").expect("YAML parses");
        let toml =
            parse(Path::new("config.toml"), "[package]\nname = \"demo\"\n").expect("TOML parses");
        assert_eq!(yaml.nodes[0].kind, "object");
        assert_eq!(toml.nodes[0].kind, "object");
    }
}
