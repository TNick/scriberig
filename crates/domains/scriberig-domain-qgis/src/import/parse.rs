//! Parses QGIS XML package structure into provider-neutral project references.

use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};

use scriberig_core::EntityId;

use crate::{QgisLayerGroup, QgisLayerPresentation, QgisLayerReference};

use super::{classify_source, derive_id};

/// Extracts project XML from standalone QGS bytes or a bounded QGZ package.
///
/// # Parameters
/// - `bytes`: Exact project file or package bytes.
///
/// # Returns
/// UTF-8 XML content from the project.
///
/// # Errors
/// Returns an error for an invalid package, missing QGS member, oversized XML, or invalid UTF-8.
pub(super) fn project_xml(bytes: &[u8]) -> Result<String, String> {
    if !bytes.starts_with(b"PK\x03\x04") {
        return std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| format!("QGS source is not UTF-8 XML: {error}"));
    }
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let name = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|entry| entry.name().to_owned())
        })
        .find(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("qgs"))
        })
        .ok_or_else(|| "QGZ package does not contain a QGS project".to_owned())?;
    let mut entry = archive.by_name(&name).map_err(|error| error.to_string())?;
    if entry.size() > 32 * 1024 * 1024 {
        return Err("QGZ project XML exceeds the 32 MiB import limit".to_owned());
    }
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|error| error.to_string())?;
    Ok(xml)
}

/// Imports ordered map-layer references and maps native IDs for group linking.
///
/// # Parameters
/// - `document`: Parsed QGIS project XML.
/// - `project_id`: Stable semantic project identity.
///
/// # Returns
/// Layer references and native-to-semantic identity mapping.
pub(super) fn read_layers(
    document: &roxmltree::Document<'_>,
    project_id: EntityId,
) -> (Vec<QgisLayerReference>, BTreeMap<String, EntityId>) {
    let mut native_layers = BTreeMap::new();
    let mut layers: Vec<QgisLayerReference> = document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "maplayer")
        .enumerate()
        .map(|(index, node)| {
            let read_child = |name: &str| {
                node.children()
                    .find(|child| child.is_element() && child.tag_name().name() == name)
                    .and_then(|child| child.text())
                    .unwrap_or_default()
                    .to_owned()
            };
            let native_id = read_child("id");
            let key = if native_id.is_empty() {
                format!("layer-reference/{index}")
            } else {
                format!("layer-reference/{native_id}")
            };
            let id = derive_id(project_id, &key);
            if !native_id.is_empty() {
                native_layers.insert(native_id.clone(), id);
            }
            let name = read_child("layername");
            let source = read_child("datasource");
            let provider = node
                .children()
                .find(|child| child.is_element() && child.tag_name().name() == "provider")
                .and_then(|child| child.text())
                .map(str::to_owned);
            let source_layer = source
                .split("|layername=")
                .nth(1)
                .map(|value| value.split('|').next().unwrap_or_default().to_owned());
            let renderer = node
                .descendants()
                .find(|child| child.is_element() && child.tag_name().name() == "renderer-v2")
                .and_then(|child| child.attribute("type"))
                .map(str::to_owned);
            let visibility = document
                .descendants()
                .find(|candidate| {
                    candidate.is_element()
                        && candidate.tag_name().name() == "layer-tree-layer"
                        && candidate.attribute("id") == Some(native_id.as_str())
                })
                .and_then(|candidate| candidate.attribute("checked"))
                .is_none_or(|checked| checked != "Qt::Unchecked" && checked != "0");
            QgisLayerReference {
                id,
                name,
                source: classify_source(source, provider),
                source_layer,
                dataset: None,
                layer: None,
                resolved_path: None,
                resolution_diagnostic: Some(
                    "dataset layer reference has not been resolved against the project scan"
                        .to_owned(),
                ),
                presentation: QgisLayerPresentation {
                    visible: visibility,
                    renderer,
                    scale_range: None,
                },
            }
        })
        .collect();
    let tree_order = document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "layer-tree-layer")
        .filter_map(|node| node.attribute("id"))
        .filter_map(|native_id| native_layers.get(native_id).copied())
        .enumerate()
        .map(|(index, layer)| (layer, index))
        .collect::<HashMap<_, _>>();
    layers.sort_by_key(|layer| tree_order.get(&layer.id).copied().unwrap_or(usize::MAX));
    (layers, native_layers)
}

/// Imports named layer groups and resolves member references.
///
/// # Parameters
/// - `document`: Parsed QGIS project XML.
/// - `project_id`: Stable semantic project identity.
/// - `native_layers`: Native layer IDs mapped to semantic references.
///
/// # Returns
/// Ordered groups with only members that resolve to imported layer references.
pub(super) fn read_groups(
    document: &roxmltree::Document<'_>,
    project_id: EntityId,
    native_layers: &BTreeMap<String, EntityId>,
) -> Vec<QgisLayerGroup> {
    document
        .descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "layer-tree-group"
                && node.attribute("name").is_some()
        })
        .enumerate()
        .map(|(index, node)| {
            let members = node
                .descendants()
                .filter(|child| child.is_element() && child.tag_name().name() == "layer-tree-layer")
                .filter_map(|child| child.attribute("id"))
                .filter_map(|native_id| native_layers.get(native_id).copied())
                .collect();
            QgisLayerGroup {
                id: derive_id(project_id, &format!("layer-group/{index}")),
                name: node.attribute("name").unwrap_or_default().to_owned(),
                layers: members,
            }
        })
        .collect()
}

/// Reads the project's declared coordinate reference system when present.
///
/// # Parameters
/// - `document`: Parsed QGIS project XML.
///
/// # Returns
/// Authored CRS metadata or `None` when the project omits it.
pub(super) fn read_project_crs(
    document: &roxmltree::Document<'_>,
) -> Option<scriberig_domain_geo::CoordinateReferenceSystem> {
    document
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "spatialrefsys")
        .map(|node| {
            let child_text = |name: &str| {
                node.children()
                    .find(|child| child.is_element() && child.tag_name().name() == name)
                    .and_then(|child| child.text())
                    .map(str::to_owned)
            };
            let authid = child_text("authid");
            let (authority, code) = authid
                .as_deref()
                .and_then(|value| value.split_once(':'))
                .map_or((None, None), |(authority, code)| {
                    (Some(authority.to_owned()), Some(code.to_owned()))
                });
            scriberig_domain_geo::CoordinateReferenceSystem {
                authority,
                code,
                definition: child_text("srid").or(authid),
            }
        })
}
