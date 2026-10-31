//! Validates internal targets declared by Open Packaging Convention relationships.

use std::io::{Cursor, Read};

use zip::ZipArchive;

/// Validates every relationship part and resolves its internal targets.
///
/// # Parameters
/// - `bytes`: Exact DOCX package bytes to inspect.
///
/// # Returns
/// Unit when relationship parts are well formed and all internal targets exist.
///
/// # Errors
/// Returns a diagnostic for malformed relationship XML, duplicate identities,
/// unsupported targets, or missing package members.
pub(crate) fn validate_relationship_targets(bytes: &[u8]) -> Result<(), String> {
    let mut package = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| format!("invalid DOCX ZIP: {error}"))?;
    let members = (0..package.len())
        .map(|index| {
            package
                .by_index(index)
                .map(|entry| entry.name().to_owned())
                .map_err(|error| format!("invalid DOCX member: {error}"))
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    let relationship_parts = members
        .iter()
        .filter(|name| is_relationship_part(name))
        .cloned()
        .collect::<Vec<_>>();
    for relation_path in relationship_parts {
        let source_path = source_part_path(&relation_path)?;
        let mut entry = package
            .by_name(&relation_path)
            .map_err(|error| format!("cannot read {relation_path}: {error}"))?;
        if entry.size() > 64 * 1024 * 1024 {
            return Err(format!("relationship part {relation_path} exceeds 64 MiB"));
        }
        let mut xml = String::new();
        entry
            .read_to_string(&mut xml)
            .map_err(|error| format!("cannot read relationship XML {relation_path}: {error}"))?;
        drop(entry);
        let document = roxmltree::Document::parse(&xml)
            .map_err(|error| format!("relationship XML {relation_path} is invalid: {error}"))?;
        let root = document.root_element();
        if root.tag_name().name() != "Relationships" {
            return Err(format!(
                "relationship part {relation_path} has no Relationships root"
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for relation in root.children().filter(roxmltree::Node::is_element) {
            if relation.tag_name().name() != "Relationship" {
                return Err(format!(
                    "relationship part {relation_path} has an unknown child"
                ));
            }
            let id = relation
                .attribute("Id")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("relationship in {relation_path} has no Id"))?;
            if !ids.insert(id) {
                return Err(format!("relationship part {relation_path} repeats Id {id}"));
            }
            relation
                .attribute("Type")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("relationship {id} in {relation_path} has no Type"))?;
            let target = relation
                .attribute("Target")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("relationship {id} in {relation_path} has no Target"))?;
            if relation.attribute("TargetMode") == Some("External") {
                continue;
            }
            let member = resolve_target(&source_path, target).map_err(|error| {
                format!("relationship {id} in {relation_path} has invalid target: {error}")
            })?;
            if !members.contains(&member) {
                return Err(format!(
                    "relationship {id} in {relation_path} targets missing package member {member}"
                ));
            }
        }
    }
    Ok(())
}

/// Identifies package relationship parts by their reserved OPC path shape.
///
/// # Parameters
/// - `path`: Package member path.
///
/// # Returns
/// `true` when `path` is an OPC relationship part.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn is_relationship_part(path: &str) -> bool {
    // ZIP package part names are case-sensitive and are not host filesystem paths.
    path.starts_with("_rels/") || (path.contains("/_rels/") && path.ends_with(".rels"))
}

/// Maps a relationship part path to the package part that owns it.
///
/// # Parameters
/// - `path`: Relationship package member path.
///
/// # Returns
/// Owning source part path, or the empty path for package-level relationships.
///
/// # Errors
/// Returns an error when the relationship path does not follow OPC naming.
fn source_part_path(path: &str) -> Result<String, String> {
    if path == "_rels/.rels" {
        return Ok(String::new());
    }
    let (directory, file) = if let Some(path) = path.strip_prefix("_rels/") {
        ("", path)
    } else {
        path.rsplit_once("/_rels/")
            .ok_or_else(|| format!("invalid relationship part path {path}"))?
    };
    let source = file
        .strip_suffix(".rels")
        .ok_or_else(|| format!("invalid relationship part path {path}"))?;
    if directory.is_empty() {
        Ok(source.to_owned())
    } else {
        Ok(format!("{directory}/{source}"))
    }
}

/// Resolves and normalizes one package-relative relationship target.
///
/// # Parameters
/// - `source`: Owning part path, or empty for package-level relationships.
/// - `target`: Authored internal relationship target.
///
/// # Returns
/// Normalized package member path.
///
/// # Errors
/// Returns an error for empty paths, unsupported URI syntax, invalid escapes,
/// or traversal above the package root.
fn resolve_target(source: &str, target: &str) -> Result<String, String> {
    let target = target.split('#').next().unwrap_or_default();
    if target.is_empty() || target.contains('?') || target.contains('\\') {
        return Err("target is empty or uses unsupported URI syntax".to_owned());
    }
    let decoded = percent_decode(target)?;
    let mut parts = if decoded.starts_with('/') {
        Vec::new()
    } else {
        source
            .rsplit_once('/')
            .map_or_else(Vec::new, |(directory, _)| {
                directory.split('/').map(str::to_owned).collect()
            })
    };
    for part in decoded.trim_start_matches('/').split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("target escapes the package root".to_owned());
                }
            }
            value => parts.push(value.to_owned()),
        }
    }
    if parts.is_empty() {
        return Err("target resolves to the package root".to_owned());
    }
    Ok(parts.join("/"))
}

/// Decodes URI percent escapes while preserving valid UTF-8 package paths.
///
/// # Parameters
/// - `value`: URI path to decode.
///
/// # Returns
/// Decoded package path.
///
/// # Errors
/// Returns an error for incomplete or non-hexadecimal percent escapes and
/// invalid UTF-8.
fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let digits = bytes
                .get(index + 1..index + 3)
                .ok_or_else(|| "incomplete percent escape".to_owned())?;
            let pair = std::str::from_utf8(digits).map_err(|error| error.to_string())?;
            decoded.push(u8::from_str_radix(pair, 16).map_err(|error| error.to_string())?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|error| error.to_string())
}
