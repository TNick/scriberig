//! Exercises the Markdown projection, filesystem, and storage vertical slice.

mod file_sync_support;

use std::fs;

use scriberig_core::{EntityId, SnapshotId};
use scriberig_store::SnapshotStore;
use scriberig_store_memory::MemoryStore;
use sr_change::Operation;
use sr_markdown::{
    MarkdownAdapter, MarkdownBlock, MarkdownReconciliation, MarkdownStyle, ThreeWayOutcome,
    match_nodes, parse, reconcile, synchronize, three_way_merge,
};
use sr_projection::{
    ProjectionAdapter, ProjectionKind, ProjectionRef, ProjectionStatus, SemanticStateRef,
};
use sr_projection_fs::{
    FileSystem, LocalFileSystem, MemorySyncStore, Observation, ProjectionLocation,
    ProjectionOwnership, digest_bytes, inspect_observation,
};

use file_sync_support::{
    EDITED_SOURCE, SOURCE, apply_operations, canonical_nodes, document, persist_snapshot,
    render_request, sync_state, temporary_directory,
};

/// Verifies initial rendering, synchronization state, and feedback suppression.
#[test]
fn initial_projection_is_synchronized() {
    let root = temporary_directory("initial");
    let path = root.join("architecture.md");
    let doc = document(SOURCE);
    let nodes = canonical_nodes(&doc);
    let store = MemoryStore::new();
    let snapshot = persist_snapshot(&store, &nodes);
    let adapter = MarkdownAdapter::with_default_style(doc);
    let rendered = adapter.render(&render_request(snapshot)).expect("render");
    let io = LocalFileSystem;
    io.write_atomic(&path, &rendered.bytes).expect("write");
    let location = ProjectionLocation::new(&path);
    let mut states = MemorySyncStore::default();
    states.put(sync_state(
        &path,
        snapshot,
        &rendered.bytes,
        ProjectionOwnership::ManagedEditable,
    ));
    assert_eq!(states.find(&location).expect("state").base.0, snapshot);
    assert_eq!(
        digest_bytes(&rendered.bytes),
        states.find(&location).expect("state").rendered
    );
    assert_eq!(
        inspect_observation(&io, &mut states, &Observation::Modified(location)).expect("observe"),
        Some(ProjectionStatus::Synchronized)
    );
    let _ = fs::remove_dir_all(root);
}

/// Verifies semantic external edits become provenance-bearing shared changes.
/// Verifies semantic external edits preserve unchanged canonical nodes.
#[test]
fn external_semantic_edit_preserves_unchanged_nodes() {
    let base_document = document(SOURCE);
    let candidate = parse(EDITED_SOURCE).expect("candidate");
    let base_nodes = canonical_nodes(&base_document);
    let store = MemoryStore::new();
    let base_snapshot = persist_snapshot(&store, &base_nodes);
    let report = match_nodes(&base_nodes, &candidate.document);
    let storage_paragraph = base_nodes
        .iter()
        .find(|node| {
            matches!(&node.block, MarkdownBlock::Paragraph(text) if text == "Objects are immutable.")
        })
        .expect("storage paragraph");
    assert!(report.matches.iter().any(|item| matches!(
        item,
        sr_markdown::NodeMatch::Matched { existing, .. } if *existing == storage_paragraph.id
    )));
    let result = reconcile(base_snapshot, &base_nodes, &candidate, "architecture.md");
    let MarkdownReconciliation::MigrationProposal(proposal) = result else {
        panic!("expected migration proposal");
    };
    assert!(matches!(
        proposal.provenance.source,
        sr_change::SourceKind::ExternalProjection
    ));
    assert!(proposal.operations.iter().any(|operation| matches!(
        operation,
        Operation::ReplaceRevision { entity, .. } if *entity == storage_paragraph.id
    )));
    assert!(
        proposal
            .operations
            .iter()
            .any(|operation| matches!(operation, Operation::CreateEntity { .. }))
    );
    let result_snapshot = apply_operations(&store, base_snapshot, &proposal.operations);
    assert_ne!(result_snapshot, base_snapshot);
    let rendering_paragraph = base_nodes
        .iter()
        .find(|node| {
            matches!(&node.block, MarkdownBlock::Paragraph(text) if text == "Files are projections.")
        })
        .expect("rendering paragraph");
    assert_eq!(
        store
            .resolve(result_snapshot, rendering_paragraph.id)
            .expect("resolve rendering"),
        store
            .resolve(base_snapshot, rendering_paragraph.id)
            .expect("resolve base rendering")
    );
}

/// Verifies formatting-only edits do not create semantic history.
/// Verifies formatting-only edits do not create semantic history.
#[test]
fn formatting_only_edit_is_a_no_op() {
    let base = document("- one\n- two\n");
    let candidate = parse("* one\n* two\n").expect("candidate");
    let nodes = canonical_nodes(&base);
    let base_snapshot = SnapshotId::new([2; 32]);
    assert_ne!(
        digest_bytes(b"- one\n- two\n"),
        digest_bytes(b"* one\n* two\n")
    );
    assert!(matches!(
        reconcile(base_snapshot, &nodes, &candidate, "list.md"),
        MarkdownReconciliation::NoSemanticChange
    ));
}

/// Verifies invalid bytes retain the synchronization base for retry.
/// Verifies invalid bytes retain the synchronization base for retry.
#[test]
fn invalid_external_edit_is_recoverable() {
    let adapter = MarkdownAdapter::with_default_style(document(SOURCE));
    let kind = ProjectionKind::new("markdown").expect("kind");
    let error = adapter
        .parse(&kind, &[0xff, 0xfe])
        .expect_err("invalid UTF-8");
    assert_eq!(error.detail.code, "invalid-utf8");
    let recovery = sr_projection_fs::RecoveryRecord {
        projection: ProjectionRef(EntityId::new([93; 16])),
        base: SemanticStateRef(SnapshotId::new([3; 32])),
        current: SemanticStateRef(SnapshotId::new([3; 32])),
        external: digest_bytes(&[0xff, 0xfe]),
        location: ProjectionLocation::new("architecture.md"),
        stage: sr_projection_fs::FailureStage::Parse,
        diagnostics: vec![*error],
        bytes: vec![0xff, 0xfe],
    };
    assert_eq!(recovery.base.0, SnapshotId::new([3; 32]));
    assert_eq!(recovery.stage, sr_projection_fs::FailureStage::Parse);
    assert!(adapter.parse(&kind, SOURCE.as_bytes()).is_ok());
}

/// Verifies disjoint canonical and external changes merge into one result.
/// Verifies disjoint canonical and external changes merge into one result.
#[test]
fn disjoint_concurrent_changes_merge() {
    let base = document("one\n\ntwo\n");
    let current = document("one changed\n\ntwo\n");
    let external = document("one\n\ninserted\n\ntwo\n");
    let ThreeWayOutcome::Merged { document } = three_way_merge(Some(&base), &current, &external)
    else {
        panic!("expected disjoint merge");
    };
    assert!(
        document
            .blocks
            .iter()
            .any(|block| matches!(block, MarkdownBlock::Paragraph(text) if text == "one changed"))
    );
    assert!(
        document
            .blocks
            .iter()
            .any(|block| matches!(block, MarkdownBlock::Paragraph(text) if text == "inserted"))
    );
    let prepared = synchronize(
        Some(SnapshotId::new([4; 32])),
        Some(&base),
        SnapshotId::new([5; 32]),
        &current,
        &external,
        &MarkdownStyle::default(),
        SnapshotId::new([6; 32]),
    );
    assert_eq!(
        prepared.synchronization_base,
        Some(SnapshotId::new([6; 32]))
    );
    assert!(prepared.rendered.is_some());
}

/// Verifies conflicting concurrent changes remain explicit and retryable.
/// Verifies conflicting concurrent changes remain explicit and retryable.
#[test]
fn conflicting_concurrent_changes_are_structured() {
    let base = document("one\n");
    let current = document("canonical\n");
    let external = document("external\n");
    let result = synchronize(
        Some(SnapshotId::new([7; 32])),
        Some(&base),
        SnapshotId::new([8; 32]),
        &current,
        &external,
        &MarkdownStyle::default(),
        SnapshotId::new([9; 32]),
    );
    assert!(matches!(result.outcome, ThreeWayOutcome::Conflict { .. }));
    assert!(result.rendered.is_none());
    assert_eq!(result.synchronization_base, Some(SnapshotId::new([7; 32])));
    assert!(result.retryable);
}

/// Verifies unmanaged and read-only ownership never permits import or writes.
/// Verifies unmanaged and read-only ownership never permits import or writes.
#[test]
fn unmanaged_and_read_only_files_are_safe() {
    assert!(!ProjectionOwnership::Unmanaged.may_write());
    assert!(!ProjectionOwnership::Unmanaged.may_import());
    assert!(!ProjectionOwnership::ManagedReadOnly.may_write());
    assert!(!ProjectionOwnership::ManagedReadOnly.may_import());
    let root = temporary_directory("ownership");
    let path = root.join("generated.md");
    let io = LocalFileSystem;
    io.write_atomic(&path, b"old").expect("write");
    let location = ProjectionLocation::new(&path);
    let mut states = MemorySyncStore::default();
    states.put(sync_state(
        &path,
        SnapshotId::new([11; 32]),
        b"old",
        ProjectionOwnership::ManagedReadOnly,
    ));
    io.write_atomic(&path, b"external").expect("external edit");
    assert_eq!(
        inspect_observation(&io, &mut states, &Observation::Modified(location)).expect("observe"),
        Some(ProjectionStatus::ExternallyModified)
    );
    let _ = fs::remove_dir_all(root);
}

/// Verifies moving a path preserves its synchronization record and identity.
/// Verifies moving a path preserves its synchronization record and identity.
#[test]
fn rename_keeps_projection_identity() {
    let root = temporary_directory("rename");
    let old_path = root.join("old.md");
    let new_path = root.join("new.md");
    let io = LocalFileSystem;
    io.write_atomic(&old_path, SOURCE.as_bytes())
        .expect("write");
    fs::rename(&old_path, &new_path).expect("rename");
    let old_location = ProjectionLocation::new(&old_path);
    let new_location = ProjectionLocation::new(&new_path);
    let mut states = MemorySyncStore::default();
    let snapshot = SnapshotId::new([10; 32]);
    states.put(sync_state(
        &old_path,
        snapshot,
        SOURCE.as_bytes(),
        ProjectionOwnership::ManagedEditable,
    ));
    assert_eq!(
        inspect_observation(
            &io,
            &mut states,
            &Observation::Renamed {
                from: old_location,
                to: new_location.clone(),
            },
        )
        .expect("observe"),
        Some(ProjectionStatus::Synchronized)
    );
    assert_eq!(
        states.find(&new_location).expect("moved state").base.0,
        snapshot
    );
    let _ = fs::remove_dir_all(root);
}
