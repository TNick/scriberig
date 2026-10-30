//! Renames one QGIS layer reference while retaining unrelated project content.

use std::io::{Cursor, Read, Write};

use roxmltree::{Document, Node};
use scriberig_core::EntityId;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::{import::derive_id, import_qgis_project};

pub(super) const MAX_PROJECT_BYTES: usize = 256 * 1024 * 1024;
const MAX_QGS_BYTES: u64 = 32 * 1024 * 1024;

/// Renames the authored map layer and its tree entries by exact semantic identity.
///
/// The writer changes only the matching `<layername>` text and corresponding
/// layer-tree `name` attributes. Other project XML and all unrelated QGZ
/// members are retained byte for byte.
///
/// # Parameters
/// - `bytes`: Exact standalone QGS or QGZ source bytes.
/// - `project`: Stable semantic project identity used during import.
/// - `reference`: Exact semantic layer-reference identity to rename.
/// - `new_name`: Non-empty display name without control characters.
///
/// # Returns
/// Rewritten project bytes with the selected map layer and tree entries renamed.
///
/// # Errors
/// Returns an error for unsupported package shape, invalid XML, ambiguous or
/// absent identity, or an invalid new name.
pub fn rename_layer_reference(
    bytes: &[u8],
    project: EntityId,
    reference: EntityId,
    new_name: &str,
) -> Result<Vec<u8>, String> {
    if bytes.len() > MAX_PROJECT_BYTES {
        return Err("QGIS project exceeds the 256 MiB edit limit".to_owned());
    }
    if new_name.trim().is_empty() || new_name.chars().any(char::is_control) {
        return Err("QGIS layer name must be non-empty and contain no control characters".into());
    }
    let (xml, package_name) = project_xml_with_name(bytes)?;
    let updated = rename_xml(&xml, project, reference, new_name)?;
    let output = match package_name {
        Some(name) => rewrite_qgz(bytes, &name, updated.as_bytes())?,
        None => updated.into_bytes(),
    };
    let reopened = import_qgis_project(&output, project)?;
    let target = reopened
        .project
        .layers
        .iter()
        .find(|layer| layer.id == reference)
        .ok_or_else(|| "renamed QGIS layer reference did not reopen".to_owned())?;
    if target.name != new_name {
        return Err("renamed QGIS layer reference did not retain the requested name".to_owned());
    }
    Ok(output)
}

/// Validates that changed QGS or QGZ bytes contain exactly one safe layer rename.
///
/// This recomputes the sole changed semantic display name and reruns the writer,
/// then requires byte equality with the staged output. It therefore checks
/// both the operation and preservation of all unrelated XML/package members.
///
/// # Parameters
/// - `before`: Exact source project bytes.
/// - `after`: Staged project bytes to validate.
///
/// # Returns
/// Unit when `after` is exactly the result of one supported layer rename.
///
/// # Errors
/// Returns an error when the output is malformed, changes other semantics, or
/// cannot be reproduced by the safe writer.
pub fn validate_layer_rename(before: &[u8], after: &[u8]) -> Result<(), String> {
    let identity = EntityId::from_bytes([0; 16]);
    let original = import_qgis_project(before, identity)?.project;
    let staged = import_qgis_project(after, identity)?.project;
    if original.title != staged.title
        || original.crs != staged.crs
        || original.groups != staged.groups
        || original.layers.len() != staged.layers.len()
    {
        return Err("QGIS project changed content beyond a layer rename".to_owned());
    }
    let mut changed = Vec::new();
    for (old_layer, new_layer) in original.layers.iter().zip(&staged.layers) {
        let mut expected = new_layer.clone();
        expected.name.clone_from(&old_layer.name);
        if expected != *old_layer {
            return Err("QGIS layer source or presentation changed during rename".to_owned());
        }
        if old_layer.name != new_layer.name {
            changed.push((old_layer.id, new_layer.name.as_str()));
        }
    }
    if changed.len() != 1 {
        return Err("QGIS candidate must rename exactly one layer reference".to_owned());
    }
    let (reference, new_name) = changed[0];
    if rename_layer_reference(before, identity, reference, new_name)?.as_slice() != after {
        return Err("QGIS candidate differs from the exact safe rename output".to_owned());
    }
    Ok(())
}

/// Extracts project XML and remembers the QGS package member when present.
///
/// # Parameters
/// - `bytes`: QGS XML or QGZ archive bytes.
///
/// # Returns
/// Project XML and optional QGS member path.
///
/// # Errors
/// Returns an error when the package has no unique QGS member or oversized XML.
pub(super) fn project_xml_with_name(bytes: &[u8]) -> Result<(String, Option<String>), String> {
    if !bytes.starts_with(b"PK\x03\x04") {
        return Ok((
            std::str::from_utf8(bytes)
                .map_err(|error| format!("QGS source is not UTF-8 XML: {error}"))?
                .to_owned(),
            None,
        ));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|entry| entry.name().to_owned())
        })
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("qgs"))
        })
        .collect::<Vec<_>>();
    if names.len() != 1 {
        return Err("QGZ package must contain exactly one QGS project".to_owned());
    }
    let name = names.into_iter().next().expect("one QGS member");
    let mut entry = archive.by_name(&name).map_err(|error| error.to_string())?;
    if entry.size() > MAX_QGS_BYTES {
        return Err("QGZ project XML exceeds the 32 MiB edit limit".to_owned());
    }
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|error| format!("QGZ project XML is not UTF-8: {error}"))?;
    Ok((xml, Some(name)))
}

/// Rewrites the exact text and attributes for one semantic layer reference.
///
/// # Parameters
/// - `xml`: Original project XML.
/// - `project`: Stable semantic project identity.
/// - `reference`: Exact layer-reference identity.
/// - `new_name`: Validated display name.
///
/// # Returns
/// XML with only the selected display-name ranges changed.
///
/// # Errors
/// Returns an error for malformed XML or non-unique layer identity.
fn rename_xml(
    xml: &str,
    project: EntityId,
    reference: EntityId,
    new_name: &str,
) -> Result<String, String> {
    let document = Document::parse(xml).map_err(|error| error.to_string())?;
    let layers = document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "maplayer")
        .enumerate()
        .filter(|(index, node)| {
            let native_id = child_text(*node, "id");
            let key = if native_id.is_empty() {
                format!("layer-reference/{index}")
            } else {
                format!("layer-reference/{native_id}")
            };
            derive_id(project, &key) == reference
        })
        .map(|(_, node)| node)
        .collect::<Vec<_>>();
    if layers.len() != 1 {
        return Err("QGIS layer identity is absent or ambiguous".to_owned());
    }
    let layer = layers[0];
    let native_id = child_text(layer, "id");
    if native_id.is_empty() {
        return Err("QGIS layer without a native ID cannot be safely renamed".to_owned());
    }
    let name_node = layer
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "layername")
        .ok_or_else(|| "QGIS map layer has no layername element".to_owned())?;
    let text_node = name_node
        .children()
        .find(Node::is_text)
        .ok_or_else(|| "QGIS layername has no direct text node".to_owned())?;
    let mut ranges = vec![(text_node.range(), escape_xml_text(new_name))];
    let tree_nodes = document
        .descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "layer-tree-layer"
                && node.attribute("id") == Some(native_id.as_str())
        })
        .collect::<Vec<_>>();
    if tree_nodes.is_empty() {
        return Err("QGIS layer has no matching layer-tree entry".to_owned());
    }
    for node in tree_nodes {
        let attribute = node
            .attribute_node("name")
            .ok_or_else(|| "matching layer-tree entry has no name attribute".to_owned())?;
        ranges.push((
            attribute_value_range(xml, attribute.range())?,
            escape_xml_attr(new_name),
        ));
    }
    ranges.sort_by_key(|(range, _)| range.start);
    if ranges
        .windows(2)
        .any(|pair| pair[0].0.end > pair[1].0.start)
    {
        return Err("QGIS layer display-name ranges overlap".to_owned());
    }
    let mut output = xml.to_owned();
    for (range, replacement) in ranges.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    Ok(output)
}

/// Reads direct text from a named child element.
///
/// # Parameters
/// - `node`: Parent XML element.
/// - `name`: Child local name.
///
/// # Returns
/// Trimmed child text or an empty string when absent.
fn child_text(node: Node<'_, '_>, name: &str) -> String {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
        .and_then(|child| child.text())
        .unwrap_or_default()
        .to_owned()
}

/// Finds the byte range occupied by an attribute's quoted value.
///
/// # Parameters
/// - `xml`: XML source containing the attribute.
/// - `range`: Attribute range including its name and quotes.
///
/// # Returns
/// Byte range of the value only.
///
/// # Errors
/// Returns an error when the parser range is malformed.
fn attribute_value_range(
    xml: &str,
    range: std::ops::Range<usize>,
) -> Result<std::ops::Range<usize>, String> {
    let attribute = xml
        .get(range.clone())
        .ok_or_else(|| "attribute range is outside project XML".to_owned())?;
    let equals = attribute
        .find('=')
        .ok_or_else(|| "attribute has no value separator".to_owned())?;
    let quote = attribute[equals + 1..]
        .trim_start()
        .chars()
        .next()
        .filter(|value| *value == '\'' || *value == '"')
        .ok_or_else(|| "attribute value is not quoted".to_owned())?;
    let quote_offset = attribute[equals + 1..]
        .find(quote)
        .ok_or_else(|| "attribute value has no opening quote".to_owned())?;
    let start = range.start + equals + 1 + quote_offset + quote.len_utf8();
    let end = range
        .end
        .checked_sub(1)
        .filter(|end| xml.as_bytes().get(*end) == Some(&(quote as u8)))
        .ok_or_else(|| "attribute value has no closing quote".to_owned())?;
    Ok(start..end)
}

/// Rebuilds a QGZ archive while replacing exactly its QGS member.
///
/// # Parameters
/// - `bytes`: Original QGZ bytes.
/// - `project_name`: Exact QGS member name to replace.
/// - `xml`: Updated QGS XML bytes.
///
/// # Returns
/// Rebuilt QGZ bytes with all other member contents retained.
///
/// # Errors
/// Returns an error for ZIP comments or read/write failures.
pub(super) fn rewrite_qgz(bytes: &[u8], project_name: &str, xml: &[u8]) -> Result<Vec<u8>, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let archive_comment = archive.comment().to_vec().into_boxed_slice();
    let zip64_comment = archive
        .zip64_comment()
        .map(|comment| comment.to_vec().into_boxed_slice());
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.set_raw_comment(archive_comment);
    writer.set_raw_zip64_comment(zip64_comment);
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_owned();
        if !entry.comment().is_empty() {
            return Err(format!("QGZ member {name} has an unsupported ZIP comment"));
        }
        let mut options = SimpleFileOptions::default()
            .compression_method(entry.compression())
            .last_modified_time(entry.last_modified().unwrap_or_default());
        if let Some(mode) = entry.unix_mode() {
            options = options.unix_permissions(mode);
        }
        if entry.is_dir() {
            writer
                .add_directory(name, options)
                .map_err(|error| error.to_string())?;
        } else {
            writer
                .start_file(name.clone(), options)
                .map_err(|error| error.to_string())?;
            if name == project_name {
                writer.write_all(xml).map_err(|error| error.to_string())?;
            } else {
                std::io::copy(&mut entry, &mut writer).map_err(|error| error.to_string())?;
            }
        }
    }
    writer
        .finish()
        .map(Cursor::into_inner)
        .map_err(|error| error.to_string())
}

/// Escapes authored text for safe insertion into XML text content.
///
/// # Parameters
/// - `value`: Authored display name.
///
/// # Returns
/// XML-escaped text.
fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escapes authored text for safe insertion into a double-quoted XML attribute.
///
/// # Parameters
/// - `value`: Authored display name.
///
/// # Returns
/// XML-escaped attribute text.
fn escape_xml_attr(value: &str) -> String {
    escape_xml_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
#[path = "rename_tests.rs"]
mod tests;
