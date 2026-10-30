//! Reads bounded rich-document trees from exact project snapshots.

use std::collections::HashMap;

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_document::{DocumentExtension, Paragraph};
use scriberig_store::{RevisionStore, SnapshotStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ArtifactCapability, ProjectError, ProjectService};

/// Maximum semantic records returned in one document view.
const MAX_DOCUMENT_VIEW_RECORDS: usize = 10_000;
/// Maximum nested table/block depth accepted by the public view.
const MAX_DOCUMENT_VIEW_DEPTH: usize = 32;

/// Describes one bounded rich-document tree at an exact snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentTreeView {
    /// Project owning the semantic document.
    pub project: ProjectId,
    /// Exact immutable snapshot used to read every record.
    pub snapshot: SnapshotId,
    /// Stable semantic document identity.
    pub document: EntityId,
    /// Edit capability of the exact source artifact, when catalogued.
    pub artifact_capability: Option<ArtifactCapability>,
    /// Named document styles available to paragraphs and inline runs.
    pub styles: Vec<DocumentStyleView>,
    /// Ordered document sections.
    pub sections: Vec<DocumentSectionView>,
}

/// Describes one named, adapter-owned style definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentStyleView {
    /// Stable style key within the source format.
    pub key: String,
    /// Human-readable style name when supplied by the source.
    pub name: Option<String>,
}

/// Represents one ordered section in a rich document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentSectionView {
    /// Stable section identity.
    pub id: EntityId,
    /// Ordered paragraphs and tables in the section.
    pub blocks: Vec<DocumentBlockView>,
}

/// Represents one semantically addressable document block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocumentBlockView {
    /// Typed paragraph including ordered inline runs and style references.
    Paragraph {
        /// Paragraph semantic value.
        paragraph: Paragraph,
    },
    /// Typed table with independently addressable rows and cells.
    Table {
        /// Table semantic value.
        table: DocumentTableView,
    },
}

/// Represents an ordered table and its semantic rows.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentTableView {
    /// Stable table identity.
    pub id: EntityId,
    /// Rows in authored order.
    pub rows: Vec<DocumentRowView>,
}

/// Represents one ordered table row and its cells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentRowView {
    /// Stable row identity.
    pub id: EntityId,
    /// Cells in authored order.
    pub cells: Vec<DocumentCellView>,
}

/// Represents one table cell and its ordered content blocks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentCellView {
    /// Stable cell identity.
    pub id: EntityId,
    /// Ordered paragraphs and nested tables in the cell.
    pub blocks: Vec<DocumentBlockView>,
}

impl ProjectService {
    /// Reads one bounded document tree from an exact active project snapshot.
    ///
    /// Every section, block, row, and cell is resolved independently so its
    /// stable semantic identity remains available to callers.
    ///
    /// # Parameters
    /// - `project`: Project owning the document.
    /// - `snapshot`: Exact active snapshot to inspect.
    /// - `document`: Stable `document.root` identity.
    ///
    /// # Returns
    /// Ordered semantic document tree with independently addressable records.
    ///
    /// # Errors
    /// Returns an ownership, stale-snapshot, missing-revision, decoding, depth,
    /// or size error.
    pub fn document_tree(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        document: EntityId,
    ) -> Result<DocumentTreeView, ProjectError> {
        let active = self.workspace(project)?.snapshot;
        if active != snapshot {
            return Err(document_error(
                "stale_document_snapshot",
                "document reads require the current exact project snapshot",
            ));
        }
        let ownership = self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_error)?;
        let source_path = ownership
            .iter()
            .find(|entity| entity.entity == document)
            .map(|entity| entity.relative_path.clone());
        let kinds = ownership
            .into_iter()
            .map(|entity| (entity.entity, entity.kind))
            .collect::<HashMap<_, _>>();
        if kinds.get(&document).map(String::as_str) != Some("document.root") {
            return Err(document_error(
                "document_not_found",
                "target is not an owned semantic document",
            ));
        }
        let mut budget = MAX_DOCUMENT_VIEW_RECORDS;
        let root = self.document_record(snapshot, document, &mut budget)?;
        let extensions: Vec<DocumentExtension> = root
            .get("extensions")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| document_error("invalid_document_extensions", &error.to_string()))?
            .unwrap_or_default();
        let styles = extensions
            .into_iter()
            .filter_map(|extension| match extension {
                DocumentExtension::Style(style) => Some(DocumentStyleView {
                    key: style.key,
                    name: style.name,
                }),
                _ => None,
            })
            .collect();
        let sections = id_list(&root, "sections")?
            .into_iter()
            .map(|section| {
                let record = self.document_record(snapshot, section, &mut budget)?;
                let blocks = self.document_blocks(
                    snapshot,
                    &kinds,
                    id_list(&record, "block_ids")?,
                    0,
                    &mut budget,
                )?;
                Ok(DocumentSectionView {
                    id: section,
                    blocks,
                })
            })
            .collect::<Result<Vec<_>, ProjectError>>()?;
        let artifact_capability = source_path
            .as_deref()
            .map(|path| self.artifact_capability_for_path(project, path))
            .transpose()?
            .flatten();
        Ok(DocumentTreeView {
            project,
            snapshot,
            document,
            artifact_capability,
            styles,
            sections,
        })
    }

    /// Resolves ordered block identities into typed semantic records.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot shared by every nested read.
    /// - `kinds`: Project ownership kinds indexed by stable entity identity.
    /// - `blocks`: Ordered semantic block identities.
    /// - `depth`: Current nested table depth.
    /// - `budget`: Remaining records allowed for this complete view.
    ///
    /// Typed paragraph and table blocks in authored order.
    ///
    /// Returns an error for unknown types, missing records, or excessive nesting.
    /// # Returns
    /// The result of the requested operation.
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    fn document_blocks(
        &self,
        snapshot: SnapshotId,
        kinds: &HashMap<EntityId, String>,
        blocks: Vec<EntityId>,
        depth: usize,
        budget: &mut usize,
    ) -> Result<Vec<DocumentBlockView>, ProjectError> {
        if depth > MAX_DOCUMENT_VIEW_DEPTH {
            return Err(document_error(
                "document_nesting_limit",
                "document table nesting exceeds the public view limit",
            ));
        }
        blocks
            .into_iter()
            .map(|id| match kinds.get(&id).map(String::as_str) {
                Some("document.paragraph") => {
                    let record = self.document_record(snapshot, id, budget)?;
                    let paragraph = serde_json::from_value(record).map_err(|error| {
                        document_error("document_record_invalid", &error.to_string())
                    })?;
                    Ok(DocumentBlockView::Paragraph { paragraph })
                }
                Some("document.table") => {
                    let table = self.document_table(snapshot, kinds, id, depth, budget)?;
                    Ok(DocumentBlockView::Table { table })
                }
                _ => Err(document_error(
                    "document_block_invalid",
                    "document contains a block type that has no public semantic view",
                )),
            })
            .collect()
    }

    /// Resolves a table, its rows, cells, and nested blocks.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot shared by every nested read.
    /// - `kinds`: Project ownership kinds indexed by stable entity identity.
    /// - `table`: Stable table identity.
    /// - `depth`: Current nested table depth.
    /// - `budget`: Remaining records allowed for this complete view.
    ///
    /// Complete ordered table view.
    ///
    /// Returns an error for missing or malformed row/cell records.
    /// # Returns
    /// The result of the requested operation.
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    fn document_table(
        &self,
        snapshot: SnapshotId,
        kinds: &HashMap<EntityId, String>,
        table: EntityId,
        depth: usize,
        budget: &mut usize,
    ) -> Result<DocumentTableView, ProjectError> {
        let record = self.document_record(snapshot, table, budget)?;
        let rows = id_list(&record, "row_ids")?
            .into_iter()
            .map(|row| {
                if kinds.get(&row).map(String::as_str) != Some("document.table_row") {
                    return Err(document_error(
                        "document_row_invalid",
                        "table row is not owned",
                    ));
                }
                let record = self.document_record(snapshot, row, budget)?;
                let cells = id_list(&record, "cell_ids")?
                    .into_iter()
                    .map(|cell| {
                        if kinds.get(&cell).map(String::as_str) != Some("document.table_cell") {
                            return Err(document_error(
                                "document_cell_invalid",
                                "table cell is not owned",
                            ));
                        }
                        let record = self.document_record(snapshot, cell, budget)?;
                        let blocks = self.document_blocks(
                            snapshot,
                            kinds,
                            id_list(&record, "block_ids")?,
                            depth + 1,
                            budget,
                        )?;
                        Ok(DocumentCellView { id: cell, blocks })
                    })
                    .collect::<Result<Vec<_>, ProjectError>>()?;
                Ok(DocumentRowView { id: row, cells })
            })
            .collect::<Result<Vec<_>, ProjectError>>()?;
        Ok(DocumentTableView { id: table, rows })
    }

    /// Reads one JSON semantic revision from an exact snapshot and charges its budget.
    ///
    /// # Parameters
    /// - `snapshot`: Exact immutable revision scope.
    /// - `entity`: Stable semantic record identity.
    /// - `budget`: Remaining records allowed for this complete view.
    ///
    /// Decoded semantic record.
    ///
    /// Returns an error when the record is missing, oversized, or malformed.
    /// # Returns
    /// The result of the requested operation.
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    fn document_record(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
        budget: &mut usize,
    ) -> Result<Value, ProjectError> {
        if *budget == 0 {
            return Err(document_error(
                "document_view_limit",
                "document exceeds the public semantic view record limit",
            ));
        }
        *budget -= 1;
        let revision = self
            .store
            .resolve(snapshot, entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                document_error("document_record_missing", "record is absent from snapshot")
            })?;
        let revision = self
            .store
            .get(revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                document_error("document_revision_missing", "record revision is missing")
            })?;
        serde_json::from_slice(revision.payload())
            .map_err(|error| document_error("document_record_invalid", &error.to_string()))
    }
}

/// Reads one ordered list of semantic IDs from a compact summary record.
///
/// # Parameters
/// - `record`: Stored summary JSON.
/// - `field`: Required list field name.
///
/// Ordered entity identities.
///
/// Returns an error when the summary is missing or malformed.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
fn id_list(record: &Value, field: &str) -> Result<Vec<EntityId>, ProjectError> {
    serde_json::from_value(
        record
            .get(field)
            .cloned()
            .ok_or_else(|| document_error("document_record_invalid", "summary field is missing"))?,
    )
    .map_err(|error| document_error("document_record_invalid", &error.to_string()))
}

/// Creates a structured document read failure.
///
/// # Parameters
/// - `code`: Stable application-facing failure category.
/// - `message`: Safe error detail without source package bytes.
///
/// Project document read error.
/// # Returns
/// The result of the requested operation.
fn document_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
