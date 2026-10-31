//! Rewrites one simple paragraph and safely copies the package members.

use std::io::{Cursor, Read, Seek, Write};

use roxmltree::{Document, Node};
use scriberig_core::EntityId;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::{MAIN_DOCUMENT, MAX_PACKAGE_BYTES, WORD_NAMESPACE, identity::derive_id};

/// Replaces the text of one paragraph only when its DOCX structure is safe.
///
/// The adapter accepts a paragraph with one text node in one run and preserves
/// all other uncompressed package member contents. Complex paragraphs remain
/// inspectable but are rejected rather than flattened or guessed.
///
/// # Parameters
/// - `bytes`: Exact source DOCX package bytes.
/// - `document_id`: Stable semantic document identity.
/// - `paragraph_id`: Stable semantic paragraph identity.
/// - `replacement`: Replacement UTF-8 text without XML control characters.
///
/// # Returns
/// Rebuilt DOCX package with the selected text node changed.
///
/// # Errors
/// Returns an error for invalid packages, unsupported paragraph structures,
/// invalid replacement text, or ZIP/XML processing failures.
pub fn replace_simple_paragraph(
    bytes: &[u8],
    document_id: EntityId,
    paragraph_id: EntityId,
    replacement: &str,
) -> Result<Vec<u8>, String> {
    rewrite_paragraph(
        bytes,
        document_id,
        paragraph_id,
        replacement,
        Cursor::new(Vec::new()),
    )
    .map(Cursor::into_inner)
}

/// Rebuilds a DOCX package into a caller-selected seekable output.
///
/// # Parameters
/// - `bytes`: Exact source package bytes.
/// - `document_id`: Stable semantic document identity.
/// - `paragraph_id`: Stable semantic paragraph identity.
/// - `replacement`: Replacement UTF-8 text.
/// - `output`: Seekable destination for the rebuilt package.
///
/// # Returns
/// The output containing the rebuilt DOCX package.
///
/// # Errors
/// Returns an error for invalid packages, unsupported paragraph structures,
/// invalid replacement text, or ZIP/XML processing failures.
pub(super) fn rewrite_paragraph<W: Seek + Write>(
    bytes: &[u8],
    document_id: EntityId,
    paragraph_id: EntityId,
    replacement: &str,
    output: W,
) -> Result<W, String> {
    if u64::try_from(bytes.len()).map_or(true, |size| size > MAX_PACKAGE_BYTES) {
        return Err("DOCX package exceeds the edit size limit".into());
    }
    if replacement.chars().any(char::is_control) {
        return Err("replacement text contains unsupported control characters".into());
    }
    if replacement.chars().next().is_some_and(char::is_whitespace)
        || replacement
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        return Err("replacement text has boundary whitespace that cannot be preserved".into());
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut document_xml = String::new();
    {
        let mut entry = archive
            .by_name(MAIN_DOCUMENT)
            .map_err(|error| format!("DOCX main document is missing: {error}"))?;
        if entry.size() > MAX_PACKAGE_BYTES {
            return Err("DOCX main document exceeds the edit size limit".into());
        }
        entry
            .read_to_string(&mut document_xml)
            .map_err(|error| error.to_string())?;
    }
    let parsed = Document::parse(&document_xml).map_err(|error| error.to_string())?;
    let body = parsed
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "body")
        .ok_or_else(|| "DOCX main document has no body".to_owned())?;
    let range = find_paragraph_text_range(body, document_id, "body", paragraph_id)?
        .ok_or_else(|| "paragraph is absent from the DOCX package".to_owned())?;
    let escaped = escape_xml_text(replacement);
    document_xml.replace_range(range, &escaped);

    let archive_comment = archive.comment().to_vec().into_boxed_slice();
    let zip64_comment = archive
        .zip64_comment()
        .map(|comment| comment.to_vec().into_boxed_slice());
    let mut writer = ZipWriter::new(output);
    writer.set_raw_comment(archive_comment);
    writer.set_raw_zip64_comment(zip64_comment);
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_owned();
        let directory = entry.is_dir();
        let compression = entry.compression();
        let modified = entry.last_modified().unwrap_or_default();
        let permissions = entry.unix_mode();
        if !entry.comment().is_empty() {
            return Err(format!("DOCX member {name} has an unsupported ZIP comment"));
        }
        let mut options = SimpleFileOptions::default()
            .compression_method(compression)
            .last_modified_time(modified);
        if let Some(mode) = permissions {
            options = options.unix_permissions(mode);
        }
        if directory {
            writer
                .add_directory(name, options)
                .map_err(|error| error.to_string())?;
        } else {
            writer
                .start_file(name.clone(), options)
                .map_err(|error| error.to_string())?;
            if name == MAIN_DOCUMENT {
                writer
                    .write_all(document_xml.as_bytes())
                    .map_err(|error| error.to_string())?;
            } else {
                std::io::copy(&mut entry, &mut writer).map_err(|error| error.to_string())?;
            }
        }
    }
    writer.finish().map_err(|error| error.to_string())
}

/// Finds a target paragraph and returns its sole safe text-node byte range.
///
/// # Parameters
/// - `node`: Parent whose direct document blocks are traversed.
/// - `parent`: Semantic parent identity used for deterministic child IDs.
/// - `path`: Structural path matching the importer.
/// - `target`: Stable target paragraph identity.
///
/// # Returns
/// Text byte range, or `None` if the target is not in this subtree.
///
/// # Errors
/// Returns an error when the matching paragraph has unsupported structure.
fn find_paragraph_text_range(
    node: Node<'_, '_>,
    parent: EntityId,
    path: &str,
    target: EntityId,
) -> Result<Option<std::ops::Range<usize>>, String> {
    for (index, child) in node.children().filter(Node::is_element).enumerate() {
        let child_path = format!("{path}/{index}");
        match child.tag_name().name() {
            "p" if derive_id(parent, &child_path) == target => {
                return safe_text_range(child).map(Some);
            }
            "tbl" => {
                if let Some(range) = find_table_paragraph(child, parent, &child_path, target)? {
                    return Ok(Some(range));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}

/// Traverses table rows and cells using the importer's stable structural paths.
///
/// # Parameters
/// - `table`: Table node from the main document.
/// - `parent`: Semantic parent identity used to derive table ID.
/// - `path`: Structural path of the table.
/// - `target`: Stable target paragraph identity.
///
/// # Returns
/// Text byte range, or `None` when the target is outside this table.
///
/// # Errors
/// Returns an error when a matching paragraph is structurally unsafe.
fn find_table_paragraph(
    table: Node<'_, '_>,
    parent: EntityId,
    path: &str,
    target: EntityId,
) -> Result<Option<std::ops::Range<usize>>, String> {
    let table_id = derive_id(parent, path);
    for (row_index, row) in table
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "tr")
        .enumerate()
    {
        let row_id = derive_id(table_id, &format!("row/{row_index}"));
        for (cell_index, cell) in row
            .children()
            .filter(|child| child.is_element() && child.tag_name().name() == "tc")
            .enumerate()
        {
            let cell_id = derive_id(row_id, &format!("cell/{cell_index}"));
            let cell_path = format!("{path}/row/{row_index}/cell/{cell_index}");
            if let Some(range) = find_paragraph_text_range(cell, cell_id, &cell_path, target)? {
                return Ok(Some(range));
            }
        }
    }
    Ok(None)
}

/// Validates the supported one-run paragraph structure and returns its text range.
///
/// # Parameters
/// - `paragraph`: Exact target paragraph XML node.
///
/// # Returns
/// Byte range of the sole text node.
///
/// # Errors
/// Returns an error when content, run, namespace, or whitespace semantics are ambiguous.
pub(super) fn safe_text_range(paragraph: Node<'_, '_>) -> Result<std::ops::Range<usize>, String> {
    let elements = paragraph
        .children()
        .filter(Node::is_element)
        .collect::<Vec<_>>();
    if elements
        .iter()
        .any(|node| !matches!(node.tag_name().name(), "pPr" | "r"))
    {
        return Err("paragraph contains unsupported structural content".into());
    }
    let runs = elements
        .into_iter()
        .filter(|node| node.tag_name().name() == "r")
        .collect::<Vec<_>>();
    if runs.len() != 1 {
        return Err("paragraph editing requires exactly one text run".into());
    }
    let run = runs[0];
    let children = run.children().filter(Node::is_element).collect::<Vec<_>>();
    if children
        .iter()
        .any(|node| !matches!(node.tag_name().name(), "rPr" | "t"))
    {
        return Err("paragraph run contains unsupported content".into());
    }
    let texts = children
        .into_iter()
        .filter(|node| node.tag_name().name() == "t")
        .collect::<Vec<_>>();
    if texts.len() != 1 || texts[0].tag_name().namespace() != Some(WORD_NAMESPACE) {
        return Err("paragraph editing requires one WordprocessingML text node".into());
    }
    let text = texts[0];
    let value = text.text().unwrap_or_default();
    if (value.chars().next().is_some_and(char::is_whitespace)
        || value.chars().next_back().is_some_and(char::is_whitespace))
        && text.attribute(("http://www.w3.org/XML/1998/namespace", "space")) != Some("preserve")
    {
        return Err("paragraph text has whitespace semantics that cannot be preserved".into());
    }
    let content = text
        .children()
        .find(Node::is_text)
        .ok_or_else(|| "paragraph text node is empty and cannot be safely replaced".to_owned())?;
    Ok(content.range())
}

/// Escapes text data for insertion into an XML text node.
///
/// # Parameters
/// - `value`: Replacement semantic text.
///
/// # Returns
/// XML-safe text content.
fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
