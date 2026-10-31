//! Reorders flat QGIS layer trees while preserving unrelated source bytes.

use std::collections::{HashMap, HashSet};

use roxmltree::{Document, Node};
use scriberig_core::EntityId;

use crate::{QgisProject, import::derive_id, import_qgis_project, rename};

/// Reorders all layers in a flat QGIS root layer tree by semantic identity.
///
/// Nested groups, duplicate layer-tree entries, and additional project-tree
/// element kinds are rejected because their ordering semantics are not part of
/// this operation.
///
/// # Parameters
/// - `bytes`: Exact standalone QGS or QGZ source bytes.
/// - `project`: Stable semantic project identity used during import.
/// - `ordered_references`: Exact permutation of all project layer identities.
///
/// # Returns
/// Rewritten QGS or QGZ bytes with only the root layer-tree entry order changed.
///
/// # Errors
/// Returns an error for unsupported tree structure, invalid identities, or
/// malformed source/package bytes.
pub fn reorder_project_layers(
    bytes: &[u8],
    project: EntityId,
    ordered_references: &[EntityId],
) -> Result<Vec<u8>, String> {
    if bytes.len() > rename::MAX_PROJECT_BYTES {
        return Err("QGIS project exceeds the 256 MiB edit limit".to_owned());
    }
    let (xml, package_name) = rename::project_xml_with_name(bytes)?;
    let updated = reorder_xml(&xml, project, ordered_references)?;
    let output = match package_name {
        Some(name) => rename::rewrite_qgz(bytes, &name, updated.as_bytes())?,
        None => updated.into_bytes(),
    };
    let reopened = import_qgis_project(&output, project)?;
    let reopened_order = reopened
        .project
        .layers
        .iter()
        .map(|layer| layer.id)
        .collect::<Vec<_>>();
    if reopened_order != ordered_references {
        return Err("QGIS layer order did not reopen with the requested permutation".to_owned());
    }
    Ok(output)
}

/// Validates that staged project bytes are exactly one supported layer reorder.
///
/// # Parameters
/// - `before`: Exact source project bytes.
/// - `after`: Staged project bytes to validate.
///
/// # Returns
/// Unit when `after` is reproducible by the flat-tree reorder writer.
///
/// # Errors
/// Returns an error when project semantics changed beyond layer order or the
/// staged bytes differ from the safe writer output.
pub fn validate_project_layer_order(before: &[u8], after: &[u8]) -> Result<(), String> {
    let identity = EntityId::from_bytes([0; 16]);
    let original = import_qgis_project(before, identity)?.project;
    let staged = import_qgis_project(after, identity)?.project;
    if !same_project_except_layer_order(&original, &staged) {
        return Err("QGIS project changed content beyond layer order".to_owned());
    }
    let order = staged
        .layers
        .iter()
        .map(|layer| layer.id)
        .collect::<Vec<_>>();
    if original
        .layers
        .iter()
        .map(|layer| layer.id)
        .eq(order.iter().copied())
    {
        return Err("QGIS candidate does not change layer order".to_owned());
    }
    if reorder_project_layers(before, identity, &order)?.as_slice() != after {
        return Err("QGIS candidate differs from the exact safe layer-order output".to_owned());
    }
    Ok(())
}

/// Reorders direct layer-tree elements and retains all surrounding XML text.
///
/// # Parameters
/// - `xml`: Original QGS XML text.
/// - `project`: Stable semantic project identity.
/// - `ordered_references`: Requested semantic layer order.
///
/// # Returns
/// XML with the root's direct layer-tree element chunks permuted.
///
/// # Errors
/// Returns an error for an unsupported tree or a non-permutation of identities.
fn reorder_xml(
    xml: &str,
    project: EntityId,
    ordered_references: &[EntityId],
) -> Result<String, String> {
    let document = Document::parse(xml).map_err(|error| error.to_string())?;
    let roots = document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "layer-tree-group")
        .filter(|node| {
            !node.parent().is_some_and(|parent| {
                parent.is_element() && parent.tag_name().name() == "layer-tree-group"
            })
        })
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return Err("QGIS layer reorder requires exactly one root layer tree".to_owned());
    }
    let root = roots[0];
    let entries = root.children().filter(Node::is_element).collect::<Vec<_>>();
    if entries.is_empty()
        || entries
            .iter()
            .any(|node| node.tag_name().name() != "layer-tree-layer")
    {
        return Err(
            "QGIS layer reorder supports a flat tree containing only layer entries".to_owned(),
        );
    }
    let native_to_semantic = semantic_layer_ids(&document, project)?;
    let mut entry_ids = Vec::with_capacity(entries.len());
    for entry in &entries {
        let native = entry
            .attribute("id")
            .ok_or_else(|| "QGIS layer-tree entry has no native layer ID".to_owned())?;
        entry_ids.push(
            native_to_semantic
                .get(native)
                .copied()
                .ok_or_else(|| "QGIS layer-tree entry references an unknown layer".to_owned())?,
        );
    }
    validate_permutation(&entry_ids, ordered_references)?;
    if entry_ids == ordered_references {
        return Err("requested QGIS layer order is unchanged".to_owned());
    }

    let root_range = root.range();
    let root_xml = xml
        .get(root_range.clone())
        .ok_or_else(|| "QGIS root layer-tree range is invalid".to_owned())?;
    let opening_end = xml_tag_end(root_xml)? + root_range.start;
    let closing_start = root_xml
        .rfind("</")
        .map(|offset| root_range.start + offset)
        .ok_or_else(|| "QGIS root layer tree has no closing tag".to_owned())?;
    if opening_end > closing_start {
        return Err("QGIS root layer-tree boundaries are invalid".to_owned());
    }
    let ranges = entries.iter().map(Node::range).collect::<Vec<_>>();
    let prefix = &xml[opening_end..ranges[0].start];
    let chunks = ranges
        .iter()
        .enumerate()
        .map(|(index, range)| {
            let end = ranges
                .get(index + 1)
                .map_or(closing_start, |next| next.start);
            xml.get(range.start..end)
                .ok_or_else(|| "QGIS layer-tree entry range is invalid".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let positions = entry_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect::<HashMap<_, _>>();
    let mut replacement = prefix.to_owned();
    for id in ordered_references {
        let index = positions[id];
        replacement.push_str(chunks[index]);
    }
    let mut output = xml.to_owned();
    output.replace_range(opening_end..closing_start, &replacement);
    Ok(output)
}

/// Derives semantic identities for map layers with authored native IDs.
///
/// # Parameters
/// - `document`: Parsed QGIS project XML.
/// - `project`: Stable semantic project identity.
///
/// # Returns
/// Native layer ID to semantic layer-reference identity mapping.
///
/// # Errors
/// Returns an error for missing or duplicated native IDs.
fn semantic_layer_ids(
    document: &Document<'_>,
    project: EntityId,
) -> Result<HashMap<String, EntityId>, String> {
    let mut layers = HashMap::new();
    for (index, node) in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "maplayer")
        .enumerate()
    {
        let native_id = node
            .children()
            .find(|child| child.is_element() && child.tag_name().name() == "id")
            .and_then(|child| child.text())
            .unwrap_or_default();
        if native_id.is_empty() {
            return Err("QGIS layer reorder requires native IDs on every map layer".to_owned());
        }
        let identity = derive_id(project, &format!("layer-reference/{native_id}"));
        if layers.insert(native_id.to_owned(), identity).is_some() {
            return Err(format!(
                "QGIS map layer native ID {native_id} is duplicated at index {index}"
            ));
        }
    }
    Ok(layers)
}

/// Requires the requested IDs to be an exact permutation of current IDs.
///
/// # Parameters
/// - `current`: Current semantic layer-reference identities.
/// - `requested`: Requested order of semantic layer-reference identities.
///
/// # Returns
/// Unit when both slices contain the same unique identities.
///
/// # Errors
/// Returns an error for duplicates, missing IDs, or foreign IDs.
fn validate_permutation(current: &[EntityId], requested: &[EntityId]) -> Result<(), String> {
    let current_set = current.iter().copied().collect::<HashSet<_>>();
    let requested_set = requested.iter().copied().collect::<HashSet<_>>();
    if current_set.len() != current.len()
        || requested_set.len() != requested.len()
        || current_set != requested_set
    {
        return Err("requested QGIS layer order must be an exact unique permutation".to_owned());
    }
    Ok(())
}

/// Finds the first unquoted closing bracket for an XML start tag.
///
/// # Parameters
/// - `element`: XML text beginning at an element start tag.
///
/// # Returns
/// Byte offset immediately after the closing bracket.
///
/// # Errors
/// Returns an error when the start tag is unterminated.
fn xml_tag_end(element: &str) -> Result<usize, String> {
    let mut quote = None;
    for (index, byte) in element.bytes().enumerate() {
        match (quote, byte) {
            (Some(active), value) if active == value => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => return Ok(index + 1),
            _ => {}
        }
    }
    Err("QGIS root layer-tree opening tag is unterminated".to_owned())
}

/// Confirms project content is identical apart from flat layer ordering.
///
/// # Parameters
/// - `before`: Project semantic model before staging.
/// - `after`: Project semantic model after staging.
///
/// # Returns
/// Whether metadata, group structure, and each layer's non-order fields match.
fn same_project_except_layer_order(before: &QgisProject, after: &QgisProject) -> bool {
    before.title == after.title
        && before.crs == after.crs
        && before.groups.len() == 1
        && after.groups.len() == 1
        && before.groups[0].id == after.groups[0].id
        && before.groups[0].name == after.groups[0].name
        && before.groups[0].layers
            == before
                .layers
                .iter()
                .map(|layer| layer.id)
                .collect::<Vec<_>>()
        && after.groups[0].layers
            == after
                .layers
                .iter()
                .map(|layer| layer.id)
                .collect::<Vec<_>>()
        && before.layers.len() == after.layers.len()
        && before.layers.iter().all(|old| {
            after
                .layers
                .iter()
                .find(|new| new.id == old.id)
                .is_some_and(|new| new == old)
        })
}

#[cfg(test)]
#[path = "layer_order_tests.rs"]
mod tests;
