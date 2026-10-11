//! Converts single-base Markdown matches into semantic change proposals.

use std::collections::BTreeMap;

use crate::{
    CandidateDocument, MarkdownBlock,
    match_nodes::{CanonicalNode, MatchReport, NodeMatch, match_nodes},
};
use scriberig_core::{EncodingVersion, EntityId, ObjectType, SnapshotId, revision_id};
use sr_change::{Operation, Provenance, SourceKind};

/// Top-level candidate blocks paired with structural paths.
type FlattenedBlocks = Vec<(Vec<usize>, MarkdownBlock)>;

/// A pure Markdown migration proposal awaiting application-layer commit.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownMigrationProposal {
    /// Exact synchronization-base snapshot.
    pub base: SnapshotId,
    /// Semantic operations to apply.
    pub operations: Vec<Operation>,
    /// Matching evidence used to create the operations.
    pub matches: MatchReport,
    /// External projection provenance.
    pub provenance: Provenance,
}

/// Structured ambiguity that prevents safe single-base reconciliation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationAmbiguity {
    /// Candidate path that could not be mapped.
    pub candidate_path: Vec<usize>,
    /// Possible existing identities.
    pub possible: Vec<EntityId>,
}

/// Result of reconciling one candidate against its unchanged canonical base.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkdownReconciliation {
    /// Candidate has no semantic effect.
    NoSemanticChange,
    /// Candidate produced an unapplied semantic proposal.
    MigrationProposal(Box<MarkdownMigrationProposal>),
    /// Candidate requires explicit user disambiguation.
    Ambiguous(Vec<ReconciliationAmbiguity>),
}

/// Reconciles one candidate against a single canonical base.
///
/// # Parameters
/// - `base_snapshot`: Exact synchronization-base snapshot.
/// - `base`: Canonical projected nodes.
/// - `candidate`: Parsed external candidate.
/// - `projection`: Stable projection identity/path metadata.
///
/// # Returns
/// A no-op, structured ambiguity, or unapplied migration proposal.
#[must_use]
pub fn reconcile(
    base_snapshot: SnapshotId,
    base: &[CanonicalNode],
    candidate: &CandidateDocument,
    projection: impl Into<String>,
) -> MarkdownReconciliation {
    let report = match_nodes(base, &candidate.document);
    let ambiguities: Vec<_> = report
        .matches
        .iter()
        .filter_map(|item| match item {
            NodeMatch::Ambiguous {
                candidate_path,
                possible,
            } => Some(ReconciliationAmbiguity {
                candidate_path: candidate_path.clone(),
                possible: possible.clone(),
            }),
            _ => None,
        })
        .collect();
    if !ambiguities.is_empty() {
        return MarkdownReconciliation::Ambiguous(ambiguities);
    }
    if report.unmatched.is_empty()
        && report
            .matches
            .iter()
            .all(|item| matches!(item, NodeMatch::Matched { .. }))
        && candidate.document.blocks.is_empty()
        && base.is_empty()
    {
        return MarkdownReconciliation::NoSemanticChange;
    }
    let operations = build_operations(base, &candidate.document, &report);
    if operations.is_empty() {
        return MarkdownReconciliation::NoSemanticChange;
    }
    let projection = projection.into();
    let mut metadata = BTreeMap::new();
    metadata.insert("projection".to_owned(), projection);
    MarkdownReconciliation::MigrationProposal(Box::new(MarkdownMigrationProposal {
        base: base_snapshot,
        operations,
        matches: report,
        provenance: Provenance {
            source: SourceKind::ExternalProjection,
            actor: None,
            workspace: None,
            projection: None,
            validation: Vec::new(),
            parents: Vec::new(),
            metadata,
        },
    }))
}

/// Builds semantic operations while preserving matched identities.
///
/// # Parameters
/// - `base`: Canonical projected nodes.
/// - `document`: Candidate semantic document.
/// - `report`: Matching evidence.
///
/// # Returns
/// Semantic operations that preserve matched identities.
fn build_operations(
    base: &[CanonicalNode],
    document: &crate::MarkdownDocument,
    report: &MatchReport,
) -> Vec<Operation> {
    let candidates = flatten_blocks(document);
    let mut operations = Vec::new();
    for item in &report.matches {
        match item {
            NodeMatch::Matched {
                candidate_path,
                existing,
                ..
            } => {
                let Some((_, block)) = candidates.iter().find(|(path, _)| path == candidate_path)
                else {
                    continue;
                };
                let Some(old) = base.iter().find(|node| node.id == *existing) else {
                    continue;
                };
                if old.block != *block {
                    operations.push(Operation::ReplaceRevision {
                        entity: *existing,
                        revision: block_revision(block),
                    });
                }
                let position = candidate_path.last().copied().unwrap_or(old.position);
                if old.position != position {
                    if let Some(collection) = old.parent {
                        operations.push(Operation::MoveCollectionMember {
                            collection,
                            member: *existing,
                            position: sr_change::CollectionPosition {
                                index: position as u64,
                            },
                        });
                    }
                }
            }
            NodeMatch::New { candidate_path } => {
                let Some((_, block)) = candidates.iter().find(|(path, _)| path == candidate_path)
                else {
                    continue;
                };
                let entity = EntityId::new(rand_bytes(candidate_path));
                operations.push(Operation::CreateEntity {
                    entity,
                    revision: block_revision(block),
                });
                if let Some(collection) = matched_parent(candidate_path, &report.matches) {
                    operations.push(Operation::InsertCollectionMember {
                        collection,
                        member: entity,
                        position: sr_change::CollectionPosition {
                            index: candidate_path.last().copied().unwrap_or(0) as u64,
                        },
                    });
                }
            }
            NodeMatch::Ambiguous { .. } => {}
        }
    }
    for entity in &report.unmatched {
        if let Some(node) = base.iter().find(|node| node.id == *entity) {
            if let Some(collection) = node.parent {
                operations.push(Operation::RemoveCollectionMember {
                    collection,
                    member: *entity,
                });
                continue;
            }
        }
        operations.push(Operation::RemoveEntity { entity: *entity });
    }
    operations
}

/// Finds the stable parent collection matched for a candidate path.
fn matched_parent(path: &[usize], matches: &[NodeMatch]) -> Option<EntityId> {
    if path.len() <= 1 {
        return None;
    }
    let parent_path = &path[..path.len() - 1];
    matches.iter().find_map(|item| match item {
        NodeMatch::Matched {
            candidate_path,
            existing,
            ..
        } if candidate_path == parent_path => Some(*existing),
        _ => None,
    })
}

/// Produces a deterministic revision for a projected block.
///
/// # Parameters
/// - `block`: Semantic block to identify.
///
/// # Returns
/// Revision identity for the block's canonical payload.
fn block_revision(block: &MarkdownBlock) -> scriberig_core::RevisionId {
    let payload = format!("{block:?}");
    revision_id(
        EncodingVersion::V1,
        &ObjectType::new("markdown.block").expect("static type"),
        payload.as_bytes(),
    )
    .expect("static canonical encoding version")
}

/// Derives a stable new entity identity from a candidate path.
///
/// # Parameters
/// - `path`: Semantic candidate path used to distinguish duplicate inserts.
///
/// # Returns
/// A deterministic 128-bit entity identity.
fn rand_bytes(path: &[usize]) -> [u8; 16] {
    let digest = blake3::hash(format!("markdown.candidate:{path:?}").as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    bytes
}

/// Flattens blocks for operation lookup.
///
/// # Parameters
/// - `document`: Candidate document.
///
/// # Returns
/// Top-level blocks paired with structural paths.
fn flatten_blocks(document: &crate::MarkdownDocument) -> FlattenedBlocks {
    let mut output = Vec::new();
    for (index, block) in document.blocks.iter().enumerate() {
        flatten_block(block, &[index], &mut output);
    }
    output
}

/// Adds one block and its nested sections to the operation lookup table.
///
/// # Parameters
/// - `block`: Semantic block to add.
/// - `path`: Structural path of the block.
/// - `output`: Destination flattened blocks.
fn flatten_block(block: &MarkdownBlock, path: &[usize], output: &mut FlattenedBlocks) {
    output.push((path.to_owned(), block.clone()));
    if let MarkdownBlock::Section(section) = block {
        for (index, child) in section.blocks.iter().enumerate() {
            let mut child_path = path.to_owned();
            child_path.push(index);
            flatten_block(child, &child_path, output);
        }
    }
}
