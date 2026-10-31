//! Probes whether exact DOCX package evidence supports the safe writer.

use std::io::{Cursor, Read};

use roxmltree::{Document, Node};
use scriberig_core::EntityId;
use zip::ZipArchive;

use super::{
    MAIN_DOCUMENT,
    identity::derive_id,
    rewrite::{rewrite_paragraph, safe_text_range},
};

/// Probes whether one supported paragraph can be rewritten with package preservation.
///
/// # Parameters
/// - `bytes`: Exact DOCX package source evidence.
/// - `document_id`: Stable semantic document identity used by paragraph IDs.
///
/// # Returns
/// True when a safe paragraph can be rewritten and the package can be rebuilt.
///
/// # Errors
/// Returns an error when package XML or ZIP processing fails.
pub(crate) fn probe_safe_paragraph_edit(
    bytes: &[u8],
    document_id: EntityId,
) -> Result<bool, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut xml = String::new();
    archive
        .by_name(MAIN_DOCUMENT)
        .map_err(|error| error.to_string())?
        .read_to_string(&mut xml)
        .map_err(|error| error.to_string())?;
    let parsed = Document::parse(&xml).map_err(|error| error.to_string())?;
    let body = parsed
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "body")
        .ok_or_else(|| "DOCX main document has no body".to_owned())?;
    let Some(paragraph) = first_safe_paragraph(body, document_id, "body") else {
        return Ok(false);
    };
    rewrite_paragraph(
        bytes,
        document_id,
        paragraph,
        "Scriberig edit probe",
        tempfile::tempfile().map_err(|error| error.to_string())?,
    )?;
    Ok(true)
}

/// Finds the first paragraph whose exact XML structure supports the safe writer.
///
/// # Parameters
/// - `node`: Parent whose direct blocks are traversed.
/// - `parent`: Semantic parent identity used to derive child IDs.
/// - `path`: Structural path matching the importer.
///
/// # Returns
/// The first safely editable paragraph identity, when one exists.
fn first_safe_paragraph(node: Node<'_, '_>, parent: EntityId, path: &str) -> Option<EntityId> {
    for (index, child) in node.children().filter(Node::is_element).enumerate() {
        let child_path = format!("{path}/{index}");
        match child.tag_name().name() {
            "p" if safe_text_range(child).is_ok() => {
                return Some(derive_id(parent, &child_path));
            }
            "tbl" => {
                if let Some(paragraph) = first_safe_table_paragraph(child, parent, &child_path) {
                    return Some(paragraph);
                }
            }
            _ => {}
        }
    }
    None
}

/// Finds the first safe paragraph nested in the rows and cells of a table.
///
/// # Parameters
/// - `table`: Exact table element from the main document XML.
/// - `parent`: Semantic parent identity used to derive the table ID.
/// - `path`: Structural path matching the importer.
///
/// # Returns
/// The first safely editable paragraph identity, when one exists.
fn first_safe_table_paragraph(
    table: Node<'_, '_>,
    parent: EntityId,
    path: &str,
) -> Option<EntityId> {
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
            if let Some(paragraph) = first_safe_paragraph(cell, cell_id, &cell_path) {
                return Some(paragraph);
            }
        }
    }
    None
}
