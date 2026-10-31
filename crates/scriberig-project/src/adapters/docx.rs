//! Imports DOCX document blocks as separately addressable project entities.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_document::{DocumentBlock, DocumentExtension, Section, Table};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};
use serde::Serialize;

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

/// Imports addressable document structure from persisted DOCX bytes.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted source DOCX.
/// - `source`: Exact source artifact entity.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// Unit after supported document entities are appended.
///
/// Returns package parsing, encoding, or semantic revision errors.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
pub(crate) fn import_docx(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let imported = scriberig_domain_docx::import_docx(
        &file.bytes,
        scriberig_domain_docx::semantic_document_id(source),
    )
    .map_err(|error| {
        adapter_diagnostics_error(file, "parse", "docx_parse_failed", &[error], true)
    })?;
    let document = &imported.document;
    add_json(
        project,
        file,
        document.id,
        "document.root",
        &DocumentSummary {
            id: document.id,
            sections: document.sections.iter().map(|section| section.id).collect(),
            extensions: document.extensions.clone(),
        },
        revisions,
        changes,
        ownership,
    )?;
    for section in &document.sections {
        import_section(project, file, section, revisions, changes, ownership)?;
    }
    Ok(())
}

/// Stores section structure and imports its blocks independently.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted source file.
/// - `section`: Semantic section to import.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// Unit after section and child records are added.
///
/// Returns encoding or semantic revision errors.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
fn import_section(
    project: ProjectId,
    file: &ProjectFileRecord,
    section: &Section,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    add_json(
        project,
        file,
        section.id,
        "document.section",
        &SectionSummary {
            id: section.id,
            block_ids: section.blocks.iter().map(block_id).collect(),
        },
        revisions,
        changes,
        ownership,
    )?;
    for block in &section.blocks {
        import_block(project, file, block, revisions, changes, ownership)?;
    }
    Ok(())
}

/// Imports one paragraph, table, or drawing and its independently addressable descendants.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted source file.
/// - `block`: Semantic block to import.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// Unit after block and child records are added.
///
/// Returns encoding or semantic revision errors.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
fn import_block(
    project: ProjectId,
    file: &ProjectFileRecord,
    block: &DocumentBlock,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    match block {
        DocumentBlock::Paragraph(paragraph) => add_json(
            project,
            file,
            paragraph.id,
            "document.paragraph",
            paragraph,
            revisions,
            changes,
            ownership,
        )?,
        DocumentBlock::Drawing(drawing) => add_json(
            project,
            file,
            drawing.id,
            "document.drawing",
            drawing,
            revisions,
            changes,
            ownership,
        )?,
        DocumentBlock::Table(table) => {
            import_table(project, file, table, revisions, changes, ownership)?;
        }
    }
    Ok(())
}

/// Imports one table and its row/cell/paragraph children without duplicating them in the table payload.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted source file.
/// - `table`: Semantic table to import.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// Unit after table descendants are stored.
///
/// Returns encoding or semantic revision errors.
/// # Returns
/// The result of the requested operation.
/// # Errors
/// Returns an error when the operation cannot be completed.
fn import_table(
    project: ProjectId,
    file: &ProjectFileRecord,
    table: &Table,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    add_json(
        project,
        file,
        table.id,
        "document.table",
        &TableSummary {
            id: table.id,
            row_ids: table.rows.iter().map(|row| row.id).collect(),
        },
        revisions,
        changes,
        ownership,
    )?;
    for row in &table.rows {
        add_json(
            project,
            file,
            row.id,
            "document.table_row",
            &RowSummary {
                id: row.id,
                cell_ids: row.cells.iter().map(|cell| cell.id).collect(),
            },
            revisions,
            changes,
            ownership,
        )?;
        for cell in &row.cells {
            add_json(
                project,
                file,
                cell.id,
                "document.table_cell",
                &CellSummary {
                    id: cell.id,
                    block_ids: cell.blocks.iter().map(block_id).collect(),
                },
                revisions,
                changes,
                ownership,
            )?;
            for block in &cell.blocks {
                import_block(project, file, block, revisions, changes, ownership)?;
            }
        }
    }
    Ok(())
}

/// Stores one typed JSON semantic record at an exact identity.
///
/// - project: Owning project identity.
/// - file: Persisted source used for error context.
/// - id: Stable semantic record identity.
/// - kind: Stable semantic type label.
/// - value: Semantic value to serialize.
/// - revisions: Destination revisions.
/// - changes: Destination snapshot changes.
/// - ownership: Destination project ownership records.
///
/// Unit after the semantic record is appended.
///
/// Returns encoding or semantic revision errors.
#[allow(clippy::too_many_arguments)]
fn add_json<T: Serialize>(
    project: ProjectId,
    file: &ProjectFileRecord,
    id: EntityId,
    kind: &str,
    value: &T,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let payload = serde_json::to_vec(value).map_err(|error| {
        adapter_diagnostics_error(
            file,
            "encode",
            "encoding_failed",
            &[error.to_string()],
            true,
        )
    })?;
    crate::support::add_entity_with_id(
        project, file, id, kind, payload, revisions, changes, ownership,
    )
}

/// Returns the identity of a paragraph, table, or drawing block.
///
/// # Parameters
/// - `block`: Semantic block.
///
/// Stable block identity.
/// # Returns
/// The result of the requested operation.
fn block_id(block: &DocumentBlock) -> EntityId {
    match block {
        DocumentBlock::Paragraph(paragraph) => paragraph.id,
        DocumentBlock::Table(table) => table.id,
        DocumentBlock::Drawing(drawing) => drawing.id,
    }
}

/// Summarizes document identity, ordered sections, and document-level extensions.
#[derive(Serialize)]
struct DocumentSummary {
    /// Stable document identity.
    id: EntityId,
    /// Section identities in authored order.
    sections: Vec<EntityId>,
    /// Typed document-level metadata, including imported named styles.
    extensions: Vec<DocumentExtension>,
}

/// Summarizes section structure without embedding child content.
#[derive(Serialize)]
struct SectionSummary {
    /// Stable section identity.
    id: EntityId,
    /// Child block identities in authored order.
    block_ids: Vec<EntityId>,
}

/// Summarizes a table without embedding all row/cell content.
#[derive(Serialize)]
struct TableSummary {
    /// Stable table identity.
    id: EntityId,
    /// Row identities in authored order.
    row_ids: Vec<EntityId>,
}

/// Summarizes one table row without embedding cell content.
#[derive(Serialize)]
struct RowSummary {
    /// Stable row identity.
    id: EntityId,
    /// Cell identities in authored order.
    cell_ids: Vec<EntityId>,
}

/// Summarizes one table cell without embedding child blocks.
#[derive(Serialize)]
struct CellSummary {
    /// Stable cell identity.
    id: EntityId,
    /// Child block identities in authored order.
    block_ids: Vec<EntityId>,
}
