//! Parses bounded DOCX document structure into the shared document domain.

use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};

use scriberig_core::EntityId;
use scriberig_domain_document::{
    Document, DocumentBlock, DocumentExtension, InlineRun, Paragraph, Section, Table, TableCell,
    TableRow, TextStyleRef,
};

#[path = "import/identity.rs"]
mod identity;
use identity::{derive_id, persistent_paragraph_id, valid_para_id};

/// Maximum uncompressed main document XML accepted by the bounded importer.
const MAX_DOCUMENT_XML_BYTES: u64 = 64 * 1024 * 1024;
/// Relationship namespace used by `WordprocessingML` hyperlink attributes.
const RELATIONSHIP_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// Reports imported DOCX semantics and conservative package capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxImport {
    /// Generic structured document semantics.
    pub document: Document,
    /// Exact digest of the original package bytes.
    pub source_digest: [u8; 32],
    /// Whether semantic output can safely replace the source package.
    pub editable: bool,
    /// Safe reason for read-only capability.
    pub diagnostic: Option<String>,
    /// Paragraph identities backed by persistent `w14:paraId` attributes.
    pub stable_paragraph_ids: BTreeSet<EntityId>,
}

/// Derives the stable semantic document identity from its source artifact.
///
/// # Parameters
/// - `source`: Stable source artifact identity.
///
/// # Returns
/// Deterministic document identity used by import and edit operations.
#[must_use]
pub fn semantic_document_id(source: EntityId) -> EntityId {
    derive_id(source, "document/root")
}

/// Imports paragraphs and tables from exact DOCX package bytes.
///
/// # Parameters
/// - `bytes`: Exact source package bytes.
/// - `document_id`: Caller-owned stable semantic document identity.
///
/// # Returns
/// Document structure, exact package digest, and probed writer capability.
///
/// # Errors
/// Returns a structured error when package or main document XML is invalid.
pub fn import_docx(bytes: &[u8], document_id: EntityId) -> Result<DocxImport, String> {
    let mut package =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut document_xml = package
        .by_name("word/document.xml")
        .map_err(|error| format!("DOCX main document is missing: {error}"))?;
    if document_xml.size() > MAX_DOCUMENT_XML_BYTES {
        return Err("DOCX main document exceeds the 64 MiB import limit".to_owned());
    }
    let mut xml = String::new();
    document_xml
        .read_to_string(&mut xml)
        .map_err(|error| error.to_string())?;
    drop(document_xml);
    let hyperlinks = hyperlink_targets(&mut package)?;
    let styles = crate::styles::read_styles(&mut package)?;
    let parsed = roxmltree::Document::parse(&xml)
        .map_err(|error| format!("DOCX main document XML is invalid: {error}"))?;
    let body = parsed
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "body")
        .ok_or_else(|| "DOCX main document has no body".to_owned())?;
    let mut stable_paragraph_ids = BTreeSet::new();
    let blocks = parse_blocks(
        body,
        document_id,
        document_id,
        "body",
        &hyperlinks,
        &mut stable_paragraph_ids,
    )?;
    let (editable, diagnostic) = match crate::edit::probe_safe_paragraph_edit(bytes, document_id) {
        Ok(true) => (
            true,
            Some(
                "Simple paragraph edits preserve package members; complex structures remain read-only"
                    .to_owned(),
            ),
        ),
        Ok(false) => (
            false,
            Some("DOCX contains no paragraph supported by the safe writer".to_owned()),
        ),
        Err(reason) => (
            false,
            Some(format!("DOCX safe-writer capability probe failed: {reason}")),
        ),
    };
    Ok(DocxImport {
        document: Document {
            id: document_id,
            sections: vec![Section {
                id: derive_id(document_id, "section/0"),
                blocks,
            }],
            lists: Vec::new(),
            drawings: Vec::new(),
            extensions: styles.into_iter().map(DocumentExtension::Style).collect(),
        },
        source_digest: *blake3::hash(bytes).as_bytes(),
        editable,
        diagnostic,
        stable_paragraph_ids,
    })
}

/// Converts ordered paragraph and table nodes into generic document blocks.
///
/// # Parameters
/// - `node`: XML parent whose direct children are considered.
/// - `parent`: Semantic parent identity.
/// - `document`: Stable document identity used for persistent paragraph IDs.
/// - `path`: Stable structural path used to derive IDs.
/// - `hyperlinks`: Relationship targets indexed by package relationship ID.
/// - `stable_paragraph_ids`: Destination set for source-backed paragraph IDs.
///
/// # Returns
/// Ordered semantic blocks.
///
/// # Errors
/// Returns an error when a supported structure is malformed.
fn parse_blocks(
    node: roxmltree::Node<'_, '_>,
    parent: EntityId,
    document: EntityId,
    path: &str,
    hyperlinks: &BTreeMap<String, String>,
    stable_paragraph_ids: &mut BTreeSet<EntityId>,
) -> Result<Vec<DocumentBlock>, String> {
    let mut blocks = Vec::new();
    for (index, child) in node
        .children()
        .filter(roxmltree::Node::is_element)
        .enumerate()
    {
        let child_path = format!("{path}/{index}");
        match child.tag_name().name() {
            "p" => blocks.push(DocumentBlock::Paragraph(parse_paragraph(
                child,
                parent,
                document,
                &child_path,
                hyperlinks,
                stable_paragraph_ids,
            )?)),
            "tbl" => blocks.push(DocumentBlock::Table(parse_table(
                child,
                parent,
                document,
                &child_path,
                hyperlinks,
                stable_paragraph_ids,
            )?)),
            _ => {}
        }
    }
    Ok(blocks)
}

/// Converts one paragraph and preserves its ordered text runs and style keys.
///
/// # Parameters
/// - `node`: Source paragraph XML element.
/// - `parent`: Stable semantic parent identity.
/// - `document`: Stable document identity used for persistent paragraph IDs.
/// - `path`: Stable structural path used to derive IDs.
/// - `hyperlinks`: Relationship targets indexed by package relationship ID.
/// - `stable_paragraph_ids`: Destination set for source-backed paragraph IDs.
///
/// # Returns
/// Addressable generic paragraph.
///
/// # Errors
/// Returns an error when a persistent paragraph identity is duplicated.
fn parse_paragraph(
    node: roxmltree::Node<'_, '_>,
    parent: EntityId,
    document: EntityId,
    path: &str,
    hyperlinks: &BTreeMap<String, String>,
    stable_paragraph_ids: &mut BTreeSet<EntityId>,
) -> Result<Paragraph, String> {
    let stable_id = node
        .attribute((identity::WORD_2010_NAMESPACE, "paraId"))
        .filter(|value| valid_para_id(value))
        .map(|value| persistent_paragraph_id(document, value));
    let id = stable_id.unwrap_or_else(|| derive_id(parent, path));
    if stable_id.is_some() && !stable_paragraph_ids.insert(id) {
        return Err("DOCX contains duplicate persistent paragraph identities".to_owned());
    }
    let style = node
        .descendants()
        .find(|child| child.is_element() && child.tag_name().name() == "pStyle")
        .and_then(|child| {
            child.attribute((
                "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                "val",
            ))
        })
        .map(|key| TextStyleRef {
            key: key.to_owned(),
            adapter: "docx".to_owned(),
        });
    let runs =
        node.children()
            .filter(roxmltree::Node::is_element)
            .flat_map(|child| match child.tag_name().name() {
                "r" => vec![(child, None)],
                "hyperlink" => {
                    let relation =
                        child
                            .attribute((RELATIONSHIP_NAMESPACE, "id"))
                            .and_then(|id| hyperlinks.get(id))
                            .cloned()
                            .or_else(|| {
                                child.attribute((
                        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                        "anchor",
                    )).map(|anchor| format!("#{anchor}"))
                            });
                    child
                        .children()
                        .filter(|run| run.is_element() && run.tag_name().name() == "r")
                        .map(move |run| (run, relation.clone()))
                        .collect()
                }
                _ => Vec::new(),
            })
            .enumerate()
            .map(|(index, (run, hyperlink))| {
                let text = run
                    .descendants()
                    .filter(|child| child.is_element() && child.tag_name().name() == "t")
                    .filter_map(|child| child.text())
                    .collect::<String>();
                let run_style = run
                    .descendants()
                    .find(|child| child.is_element() && child.tag_name().name() == "rStyle")
                    .and_then(|child| {
                        child.attribute((
                            "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                            "val",
                        ))
                    })
                    .map(|key| TextStyleRef {
                        key: key.to_owned(),
                        adapter: "docx".to_owned(),
                    });
                InlineRun {
                    id: derive_id(id, &format!("run/{index}")),
                    text,
                    style: run_style,
                    hyperlink,
                }
            })
            .collect();
    Ok(Paragraph { id, runs, style })
}

/// Converts a `WordprocessingML` table to ordered generic rows and cells.
///
/// # Parameters
/// - `node`: Source table XML element.
/// - `parent`: Stable semantic parent identity.
/// - `document`: Stable document identity used for persistent paragraph IDs.
/// - `path`: Stable structural path used to derive IDs.
/// - `hyperlinks`: Relationship targets indexed by package relationship ID.
/// - `stable_paragraph_ids`: Destination set for source-backed paragraph IDs.
///
/// # Returns
/// Addressable generic table.
///
/// # Errors
/// Returns an error when a table child cannot be parsed.
fn parse_table(
    node: roxmltree::Node<'_, '_>,
    parent: EntityId,
    document: EntityId,
    path: &str,
    hyperlinks: &BTreeMap<String, String>,
    stable_paragraph_ids: &mut BTreeSet<EntityId>,
) -> Result<Table, String> {
    let table_id = derive_id(parent, path);
    let mut rows = Vec::new();
    for (row_index, row) in node
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "tr")
        .enumerate()
    {
        let row_id = derive_id(table_id, &format!("row/{row_index}"));
        let mut cells = Vec::new();
        for (cell_index, cell) in row
            .children()
            .filter(|child| child.is_element() && child.tag_name().name() == "tc")
            .enumerate()
        {
            let cell_id = derive_id(row_id, &format!("cell/{cell_index}"));
            cells.push(TableCell {
                id: cell_id,
                blocks: parse_blocks(
                    cell,
                    cell_id,
                    document,
                    &format!("{path}/row/{row_index}/cell/{cell_index}"),
                    hyperlinks,
                    stable_paragraph_ids,
                )?,
            });
        }
        rows.push(TableRow { id: row_id, cells });
    }
    Ok(Table { id: table_id, rows })
}

/// Resolves external hyperlink relationship IDs from the DOCX package.
///
/// - package: Open package containing optional document relationships.
///
/// Relationship IDs mapped to their authored external targets.
///
/// Returns an error when the relationship XML exists but is malformed.
fn hyperlink_targets<R: Read + std::io::Seek>(
    package: &mut zip::ZipArchive<R>,
) -> Result<BTreeMap<String, String>, String> {
    let Ok(mut relationships) = package.by_name("word/_rels/document.xml.rels") else {
        return Ok(BTreeMap::new());
    };
    if relationships.size() > MAX_DOCUMENT_XML_BYTES {
        return Err("DOCX relationship part exceeds the 64 MiB import limit".to_owned());
    }
    let mut xml = String::new();
    relationships
        .read_to_string(&mut xml)
        .map_err(|error| error.to_string())?;
    drop(relationships);
    let parsed = roxmltree::Document::parse(&xml)
        .map_err(|error| format!("DOCX document relationship XML is invalid: {error}"))?;
    Ok(parsed
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "Relationship")
        .filter(|node| {
            node.attribute("Type")
                .is_some_and(|value| value.ends_with("/hyperlink"))
        })
        .filter(|node| node.attribute("TargetMode") == Some("External"))
        .filter_map(|node| {
            Some((
                node.attribute("Id")?.to_owned(),
                node.attribute("Target")?.to_owned(),
            ))
        })
        .collect())
}
