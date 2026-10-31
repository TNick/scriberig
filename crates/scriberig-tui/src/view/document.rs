//! Normalizes local document trees into the stable protocol view model.

use scriberig_project::ArtifactCapability;
use scriberig_protocol::{
    ArtifactEditCapability, DocumentBlockView as WireBlock, DocumentCellView as WireCell,
    DocumentInlineRunView, DocumentInspectResponse, DocumentParagraphView, DocumentRowView,
    DocumentSectionView, DocumentStyleView, DocumentTableView, DocumentTextStyleRef,
};

/// Converts an application document tree to the shared transport-neutral view shape.
///
/// # Parameters
/// - `tree`: Exact-snapshot document tree returned by the shared project service.
///
/// # Returns
/// Protocol-shaped document content with stable IDs and fidelity metadata.
pub fn from_application(tree: scriberig_project::DocumentTreeView) -> DocumentInspectResponse {
    DocumentInspectResponse {
        project: tree.project.to_string(),
        snapshot: tree.snapshot.to_string(),
        document: tree.document.to_string(),
        artifact_capability: tree.artifact_capability.map(map_capability),
        styles: tree
            .styles
            .into_iter()
            .map(|style| DocumentStyleView {
                key: style.key,
                name: style.name,
            })
            .collect(),
        sections: tree.sections.into_iter().map(map_section).collect(),
    }
}

/// Converts one ordered semantic document section.
///
/// # Parameters
/// - `section`: Application section with its ordered semantic blocks.
///
/// # Returns
/// Protocol section preserving block order and IDs.
fn map_section(section: scriberig_project::DocumentSectionView) -> DocumentSectionView {
    DocumentSectionView {
        id: section.id.to_string(),
        blocks: section.blocks.into_iter().map(map_block).collect(),
    }
}

/// Converts a typed paragraph or table block recursively.
///
/// # Parameters
/// - `block`: Application block from the bounded document tree.
///
/// # Returns
/// Protocol block preserving semantic identity and structure.
fn map_block(block: scriberig_project::DocumentBlockView) -> WireBlock {
    match block {
        scriberig_project::DocumentBlockView::Paragraph { paragraph } => WireBlock::Paragraph {
            paragraph: DocumentParagraphView {
                id: paragraph.id.to_string(),
                runs: paragraph
                    .runs
                    .into_iter()
                    .map(|run| DocumentInlineRunView {
                        id: run.id.to_string(),
                        text: run.text,
                        style: run.style.map(map_style),
                        hyperlink: run.hyperlink,
                    })
                    .collect(),
                style: paragraph.style.map(map_style),
            },
        },
        scriberig_project::DocumentBlockView::Table { table } => WireBlock::Table {
            table: DocumentTableView {
                id: table.id.to_string(),
                rows: table
                    .rows
                    .into_iter()
                    .map(|row| DocumentRowView {
                        id: row.id.to_string(),
                        cells: row
                            .cells
                            .into_iter()
                            .map(|cell| WireCell {
                                id: cell.id.to_string(),
                                blocks: cell.blocks.into_iter().map(map_block).collect(),
                            })
                            .collect(),
                    })
                    .collect(),
            },
        },
    }
}

/// Converts one adapter-owned style reference.
///
/// # Parameters
/// - `style`: Stable style key and owning adapter.
///
/// # Returns
/// Protocol style reference without adapter-specific interpretation.
fn map_style(style: scriberig_domain_document::TextStyleRef) -> DocumentTextStyleRef {
    DocumentTextStyleRef {
        key: style.key,
        adapter: style.adapter,
    }
}

/// Maps project edit capability into the stable protocol enum.
///
/// # Parameters
/// - `capability`: Capability assessed for this exact source artifact.
///
/// # Returns
/// Equivalent protocol capability label.
fn map_capability(capability: ArtifactCapability) -> ArtifactEditCapability {
    match capability {
        ArtifactCapability::Unsupported => ArtifactEditCapability::Unsupported,
        ArtifactCapability::InspectOnly => ArtifactEditCapability::InspectOnly,
        ArtifactCapability::ReadOnlyDerived => ArtifactEditCapability::ReadOnlyDerived,
        ArtifactCapability::LosslessEditable => ArtifactEditCapability::LosslessEditable,
        ArtifactCapability::LossAwareEditable => ArtifactEditCapability::LossAwareEditable,
        ArtifactCapability::EditableWithOpaquePreservation => {
            ArtifactEditCapability::EditableWithOpaquePreservation
        }
        ArtifactCapability::RestrictedExternalProviderEdit => {
            ArtifactEditCapability::RestrictedExternalProviderEdit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtifactCapability, ArtifactEditCapability, from_application};
    use scriberig_core::{EntityId, ProjectId, SnapshotId};

    /// Keeps nested paragraph and table identities when normalizing local trees.
    #[test]
    fn normalizes_nested_document_semantics_and_fidelity() {
        let document = EntityId::from_bytes([1; 16]);
        let paragraph = EntityId::from_bytes([2; 16]);
        let run = EntityId::from_bytes([3; 16]);
        let table = EntityId::from_bytes([4; 16]);
        let row = EntityId::from_bytes([5; 16]);
        let cell = EntityId::from_bytes([6; 16]);
        let nested_paragraph = EntityId::from_bytes([7; 16]);
        let tree = scriberig_project::DocumentTreeView {
            project: ProjectId::from_bytes([8; 16]),
            snapshot: SnapshotId::new([9; 32]),
            document,
            artifact_capability: Some(ArtifactCapability::EditableWithOpaquePreservation),
            styles: vec![scriberig_project::DocumentStyleView {
                key: "BodyText".to_owned(),
                name: Some("Body Text".to_owned()),
            }],
            sections: vec![scriberig_project::DocumentSectionView {
                id: EntityId::from_bytes([10; 16]),
                blocks: vec![
                    scriberig_project::DocumentBlockView::Paragraph {
                        paragraph: scriberig_domain_document::Paragraph {
                            id: paragraph,
                            runs: vec![scriberig_domain_document::InlineRun {
                                id: run,
                                text: "Overview".to_owned(),
                                style: None,
                                hyperlink: None,
                            }],
                            style: None,
                        },
                    },
                    scriberig_project::DocumentBlockView::Table {
                        table: scriberig_project::DocumentTableView {
                            id: table,
                            rows: vec![scriberig_project::DocumentRowView {
                                id: row,
                                cells: vec![scriberig_project::DocumentCellView {
                                    id: cell,
                                    blocks: vec![scriberig_project::DocumentBlockView::Paragraph {
                                        paragraph: scriberig_domain_document::Paragraph {
                                            id: nested_paragraph,
                                            runs: Vec::new(),
                                            style: None,
                                        },
                                    }],
                                }],
                            }],
                        },
                    },
                ],
            }],
        };

        let response = from_application(tree);
        assert_eq!(response.document, document.to_string());
        assert_eq!(response.styles[0].key, "BodyText");
        assert_eq!(
            response.artifact_capability,
            Some(ArtifactEditCapability::EditableWithOpaquePreservation)
        );
        assert!(matches!(
            &response.sections[0].blocks[0],
            scriberig_protocol::DocumentBlockView::Paragraph { paragraph: value }
                if value.id == paragraph.to_string() && value.runs[0].id == run.to_string()
        ));
        assert!(matches!(
            &response.sections[0].blocks[1],
            scriberig_protocol::DocumentBlockView::Table { table: value }
                if value.id == table.to_string()
                    && value.rows[0].id == row.to_string()
                    && value.rows[0].cells[0].id == cell.to_string()
                    && matches!(&value.rows[0].cells[0].blocks[0],
                        scriberig_protocol::DocumentBlockView::Paragraph { paragraph }
                        if paragraph.id == nested_paragraph.to_string())
        ));
    }
}
