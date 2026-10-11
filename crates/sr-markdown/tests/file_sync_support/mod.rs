//! Shared fixtures for the Markdown projection integration tests.

use std::fs;
use std::path::{Path, PathBuf};

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RevisionId, SnapshotId, revision_id};
use scriberig_store::{RevisionStore, SnapshotChange, SnapshotStore};
use scriberig_store_memory::MemoryStore;
use sr_change::Operation;
use sr_markdown::{CanonicalNode, MarkdownBlock, MarkdownDocument, parse};
use sr_projection::{
    ProjectionKind, ProjectionRef, ProjectionSpecRef, ProjectionStatus, RenderRequest,
    RendererVersion, RenderingContext, SemanticScopeRef, SemanticStateRef, StyleRef, StyleValue,
};
use sr_projection_fs::{ProjectionLocation, ProjectionOwnership, SyncState, digest_bytes};

pub const SOURCE: &str = "# Architecture\n\n## Storage\n\nObjects are immutable.\n\n## Rendering\n\nFiles are projections.\n";
pub const EDITED_SOURCE: &str = "# Architecture\n\n## Storage\n\nObjects are immutable and content addressed.\n\n## Sessions\n\nSessions use the same object store.\n\n## Rendering\n\nFiles are projections.\n";

/// Parses a Markdown fixture into its semantic document.
///
/// # Parameters
/// - `source`: Markdown fixture source.
///
/// # Returns
/// Parsed semantic document.
pub fn document(source: &str) -> MarkdownDocument {
    parse(source).expect("Markdown fixture").document
}

/// Creates a unique temporary directory for one integration case.
///
/// # Parameters
/// - `label`: Test case label.
///
/// # Returns
/// Created temporary directory path.
pub fn temporary_directory(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("scriberig-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temporary directory");
    path
}

/// Computes a deterministic canonical revision for one semantic value.
///
/// # Parameters
/// - `value`: Semantic value to identify.
///
/// # Returns
/// Content-derived revision identity.
pub fn revision(value: &impl std::fmt::Debug) -> RevisionId {
    let payload = format!("{value:?}");
    revision_id(
        EncodingVersion::V1,
        &ObjectType::new("markdown.block").expect("object type"),
        payload.as_bytes(),
    )
    .expect("static canonical encoding version")
}

/// Flattens a document into canonical nodes with stable fixture identities.
///
/// # Parameters
/// - `document`: Semantic document to flatten.
///
/// # Returns
/// Canonical nodes in structural order.
pub fn canonical_nodes(document: &MarkdownDocument) -> Vec<CanonicalNode> {
    let mut nodes = Vec::new();
    let mut next = 1u8;
    for (position, block) in document.blocks.iter().enumerate() {
        flatten_node(block, None, position, &mut next, &mut nodes);
    }
    nodes
}

/// Adds one block and its nested children to a canonical fixture.
///
/// # Parameters
/// - `block`: Semantic block to add.
/// - `parent`: Optional canonical parent identity.
/// - `position`: Position within the parent collection.
/// - `next`: Next fixture identity byte.
/// - `nodes`: Destination canonical nodes.
pub fn flatten_node(
    block: &MarkdownBlock,
    parent: Option<EntityId>,
    position: usize,
    next: &mut u8,
    nodes: &mut Vec<CanonicalNode>,
) {
    let id = EntityId::new([*next; 16]);
    *next = next.saturating_add(1);
    nodes.push(CanonicalNode {
        id,
        parent,
        block: block.clone(),
        position,
    });
    if let MarkdownBlock::Section(section) = block {
        for (child_position, child) in section.blocks.iter().enumerate() {
            flatten_node(child, Some(id), child_position, next, nodes);
        }
    }
}

/// Persists fixture revisions and returns the corresponding immutable snapshot.
///
/// # Parameters
/// - `store`: In-memory storage backend.
/// - `nodes`: Canonical nodes to persist.
///
/// # Returns
/// Immutable snapshot containing the nodes.
pub fn persist_snapshot(store: &MemoryStore, nodes: &[CanonicalNode]) -> SnapshotId {
    let mut changes = Vec::new();
    for node in nodes {
        let value = revision(&node.block);
        store
            .put(
                scriberig_store::Revision::new(
                    EncodingVersion::V1,
                    ObjectType::new("markdown.block").expect("object type"),
                    format!("{:?}", node.block).into_bytes(),
                )
                .expect("revision"),
            )
            .expect("revision");
        changes.push(SnapshotChange::Set(node.id, value));
    }
    store
        .apply_changes(store.empty_snapshot(), &changes)
        .expect("snapshot")
}

/// Applies storage-level operations from a Markdown migration proposal.
///
/// # Parameters
/// - `store`: In-memory storage backend.
/// - `base`: Base snapshot for the migration.
/// - `operations`: Shared semantic operations to apply.
///
/// # Returns
/// Resulting immutable snapshot.
pub fn apply_operations(
    store: &MemoryStore,
    base: SnapshotId,
    operations: &[Operation],
) -> SnapshotId {
    let changes = operations
        .iter()
        .filter_map(|operation| match operation {
            Operation::CreateEntity { entity, revision }
            | Operation::ReplaceRevision { entity, revision } => {
                Some(SnapshotChange::Set(*entity, *revision))
            }
            Operation::RemoveEntity { entity } => Some(SnapshotChange::Remove(*entity)),
            _ => None,
        })
        .collect::<Vec<_>>();
    store
        .apply_changes(base, &changes)
        .expect("migration snapshot")
}

/// Creates a generic Markdown render request for an exact snapshot.
///
/// # Parameters
/// - `snapshot`: Exact semantic snapshot used for rendering.
///
/// # Returns
/// Generic render request.
pub fn render_request(snapshot: SnapshotId) -> RenderRequest {
    RenderRequest {
        specification: ProjectionSpecRef(EntityId::new([90; 16])),
        scope: SemanticScopeRef {
            entity: EntityId::new([91; 16]),
            revision: None,
        },
        base_state: SemanticStateRef(snapshot),
        kind: ProjectionKind::new("markdown").expect("kind"),
        style: StyleRef::Value(StyleValue::new(b"default".to_vec())),
        renderer: RendererVersion::new("test"),
        context: RenderingContext::default(),
    }
}

/// Creates and records one filesystem synchronization state.
///
/// # Parameters
/// - `path`: Physical projection path.
/// - `snapshot`: Synchronization-base snapshot.
/// - `bytes`: Last successfully rendered bytes.
/// - `ownership`: Projection ownership policy.
///
/// # Returns
/// Synchronization state record.
pub fn sync_state(
    path: &Path,
    snapshot: SnapshotId,
    bytes: &[u8],
    ownership: ProjectionOwnership,
) -> SyncState {
    SyncState {
        projection: ProjectionRef(EntityId::new([92; 16])),
        location: ProjectionLocation::new(path),
        ownership,
        base: SemanticStateRef(snapshot),
        rendered: digest_bytes(bytes),
        observed: None,
        status: ProjectionStatus::Synchronized,
    }
}
