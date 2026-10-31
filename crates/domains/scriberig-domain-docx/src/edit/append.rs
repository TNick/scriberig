//! Appends a simple paragraph while preserving unrelated DOCX package members.

use std::io::{Cursor, Read, Seek, Write};

use roxmltree::Document;
use scriberig_core::EntityId;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::{MAIN_DOCUMENT, MAX_PACKAGE_BYTES, WORD_NAMESPACE, identity::derive_id};

/// Appends one plain-text paragraph to the exact DOCX document section.
///
/// The writer supports the imported single-section shape. It inserts before
/// `w:sectPr` when present, preserving the required final section-properties
/// position and every existing block's structural identity.
///
/// # Parameters
/// - `bytes`: Exact source DOCX package bytes.
/// - `document_id`: Stable semantic document identity.
/// - `section_id`: Exact imported section identity.
/// - `text`: New paragraph text without boundary whitespace or control characters.
///
/// # Returns
/// Rebuilt DOCX package with one appended semantic paragraph.
///
/// # Errors
/// Returns an error for a stale section, invalid package, unsupported text, or
/// package rewrite failure.
pub fn append_simple_paragraph(
    bytes: &[u8],
    document_id: EntityId,
    section_id: EntityId,
    text: &str,
) -> Result<Vec<u8>, String> {
    if derive_id(document_id, "section/0") != section_id {
        return Err("section identity is not the imported DOCX section".to_owned());
    }
    if u64::try_from(bytes.len()).map_or(true, |size| size > MAX_PACKAGE_BYTES) {
        return Err("DOCX package exceeds the edit size limit".to_owned());
    }
    validate_text(text)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut document_xml = read_main_document(&mut archive)?;
    let parsed = Document::parse(&document_xml).map_err(|error| error.to_string())?;
    let body = parsed
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "body")
        .ok_or_else(|| "DOCX main document has no body".to_owned())?;
    if body.tag_name().namespace() != Some(WORD_NAMESPACE) {
        return Err("DOCX body is outside the WordprocessingML namespace".to_owned());
    }
    let insert_at = body
        .children()
        .find(|child| child.is_element() && child.tag_name().name() == "sectPr")
        .map_or_else(
            || body_close_offset(&document_xml, body.range()),
            |node| node.range().start,
        );
    let paragraph = format!(
        "<w:p xmlns:w=\"{WORD_NAMESPACE}\"><w:r><w:t>{}</w:t></w:r></w:p>",
        escape_xml_text(text)
    );
    document_xml.insert_str(insert_at, &paragraph);
    rewrite_package(archive, &document_xml)
}

/// Validates text supported by the simple paragraph writer.
///
/// # Parameters
/// - `text`: New paragraph text.
///
/// # Returns
/// Unit when the text survives XML and Word whitespace handling unchanged.
///
/// # Errors
/// Returns an error for control characters or boundary whitespace.
fn validate_text(text: &str) -> Result<(), String> {
    if text.chars().any(char::is_control) {
        return Err("paragraph text contains unsupported control characters".to_owned());
    }
    if text.chars().next().is_some_and(char::is_whitespace)
        || text.chars().next_back().is_some_and(char::is_whitespace)
    {
        return Err("paragraph text has boundary whitespace that cannot be preserved".to_owned());
    }
    Ok(())
}

/// Reads the bounded main document XML from a package.
///
/// # Parameters
/// - `archive`: Open package containing the main document member.
///
/// # Returns
/// UTF-8 main document XML.
///
/// # Errors
/// Returns an error when the part is absent, oversized, or invalid UTF-8.
fn read_main_document<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<String, String> {
    let mut entry = archive
        .by_name(MAIN_DOCUMENT)
        .map_err(|error| format!("DOCX main document is missing: {error}"))?;
    if entry.size() > MAX_PACKAGE_BYTES {
        return Err("DOCX main document exceeds the edit size limit".to_owned());
    }
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|error| error.to_string())?;
    Ok(xml)
}

/// Finds the closing-tag offset for the parsed `WordprocessingML` body.
///
/// # Parameters
/// - `xml`: Exact main document XML.
/// - `body_range`: Parser-provided byte range of the body element.
///
/// # Returns
/// Byte offset before the closing body tag.
///
/// # Panics
/// Panics only when a successfully parsed XML body has no closing tag range.
fn body_close_offset(xml: &str, body_range: std::ops::Range<usize>) -> usize {
    let body = &xml[body_range.clone()];
    let close = body
        .rfind("</")
        .expect("parsed body element has a closing tag");
    body_range.start + close
}

/// Rewrites the main document and copies all other package members unchanged.
///
/// # Parameters
/// - `archive`: Open source package.
/// - `document_xml`: Updated main document XML.
///
/// # Returns
/// Rebuilt package bytes.
///
/// # Errors
/// Returns an error for unsupported ZIP metadata or package write failures.
fn rewrite_package<R: Read + Seek>(
    mut archive: ZipArchive<R>,
    document_xml: &str,
) -> Result<Vec<u8>, String> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
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
    writer
        .finish()
        .map(std::io::Cursor::into_inner)
        .map_err(|error| error.to_string())
}

/// Escapes semantic text for an XML text node.
///
/// # Parameters
/// - `text`: Authored paragraph text.
///
/// # Returns
/// XML-safe text content.
fn escape_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
