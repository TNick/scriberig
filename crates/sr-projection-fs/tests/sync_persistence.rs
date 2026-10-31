//! Verifies synchronization baselines survive coordinator restart.

use std::fs;

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    FidelityAssessment, ProjectionAdapterIdentity, ProjectionKind, ProjectionRef,
    ProjectionSpecRef, ProjectionStatus, ReconciliationBaseline, RenderRequest, RendererVersion,
    RenderingContext, ResolvedStyle, SemanticScopeRef, SemanticStateRef, StyleValue,
};
use sr_projection_fs::{
    LocalFileSystem, MemorySyncStore, Observation, ProjectionLocation, ProjectionOwnership,
    SyncCoordinator, SyncState,
};

/// Creates a synchronization state with a complete, exact rendered baseline.
///
/// - `path`: Fixture source file and matching projection location.
/// - `baseline_bytes`: Exact rendered representation to retain.
///
/// # Returns
/// A state suitable for persistence and restart verification.
fn state(path: &std::path::Path, baseline_bytes: &[u8]) -> SyncState {
    let entity = EntityId::from_bytes([31; 16]);
    let snapshot = SnapshotId::new([32; 32]);
    let request = RenderRequest {
        specification: ProjectionSpecRef(EntityId::from_bytes([33; 16])),
        scope: SemanticScopeRef {
            entity,
            revision: None,
        },
        base_state: SemanticStateRef(snapshot),
        kind: ProjectionKind::new("test-source").expect("projection kind"),
        style: ResolvedStyle::inline(StyleValue::new(b"indent=two".to_vec())),
        renderer: RendererVersion("fixture-v1".to_owned()),
        context: RenderingContext::default(),
    };
    let rendered = sr_projection::RenderedProjection::from_bytes(
        baseline_bytes.to_vec(),
        &request,
        Some(ProjectionRef(EntityId::from_bytes([34; 16]))),
    );
    let baseline = ReconciliationBaseline::new(
        rendered,
        ProjectionAdapterIdentity::new("fixture", "1").expect("adapter identity"),
        FidelityAssessment::lossless(),
    )
    .expect("exact baseline");
    SyncState::new(
        ProjectionRef(EntityId::from_bytes([34; 16])),
        baseline,
        ProjectionLocation::new(path),
        ProjectionOwnership::ManagedEditable,
        SemanticStateRef(snapshot),
        ProjectionStatus::Synchronized,
    )
}

/// Reopens the exact baseline after an observation updates synchronized state.
#[test]
fn coordinator_restart_retains_exact_baseline_and_observation_state() {
    let root =
        std::env::temp_dir().join(format!("scriberig-sync-persistence-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    let source = root.join("source.rs");
    let persistence = root.join("sync-state.json");
    fs::write(&source, b"baseline output").expect("initial source");
    let mut coordinator = SyncCoordinator::open_persistent(LocalFileSystem, &persistence)
        .expect("open persistent coordinator");
    coordinator
        .register_state(state(&source, b"baseline output"))
        .expect("persist initial baseline");

    fs::write(&source, b"edited representation").expect("external edit");
    let observed = coordinator
        .observe(&Observation::Modified(ProjectionLocation::new(&source)))
        .expect("observe changed bytes")
        .expect("tracked path");
    assert_eq!(observed.status, ProjectionStatus::ImportPending);
    drop(coordinator);

    let reopened = SyncCoordinator::open_persistent(LocalFileSystem, &persistence)
        .expect("reopen persistent coordinator");
    let retained = reopened
        .states()
        .find(&ProjectionLocation::new(&source))
        .expect("persisted projection state");
    assert_eq!(retained.baseline.rendered.bytes, b"baseline output");
    assert_eq!(
        retained.baseline.rendered.style,
        state(&source, b"baseline output").baseline.rendered.style
    );
    assert_eq!(retained.status, ProjectionStatus::ImportPending);
    assert!(retained.observed.is_some());
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Reopens the exact replacement output after a successful managed write.
#[test]
fn successful_rendered_write_persists_replacement_baseline() {
    let root = std::env::temp_dir().join(format!("scriberig-sync-write-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    let source = root.join("source.rs");
    let persistence = root.join("sync-state.json");
    fs::write(&source, b"baseline output").expect("initial source");
    let initial = state(&source, b"baseline output");
    let projection = initial.projection;
    let mut coordinator = SyncCoordinator::open_persistent(LocalFileSystem, &persistence)
        .expect("open persistent coordinator");
    coordinator
        .register_state(initial)
        .expect("persist initial baseline");

    coordinator
        .write_rendered(
            projection,
            b"replacement output",
            SemanticStateRef(SnapshotId::new([35; 32])),
        )
        .expect("write and persist replacement");
    drop(coordinator);

    let reopened = SyncCoordinator::open_persistent(LocalFileSystem, &persistence)
        .expect("reopen persistent coordinator");
    let retained = reopened
        .states()
        .find(&ProjectionLocation::new(&source))
        .expect("persisted projection state");
    assert_eq!(retained.baseline.rendered.bytes, b"replacement output");
    assert_eq!(
        fs::read(&source).expect("materialized source"),
        b"replacement output"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Reports invalid persisted bytes instead of silently starting with no baselines.
#[test]
fn invalid_persistent_state_fails_closed() {
    let root = std::env::temp_dir().join(format!("scriberig-sync-corrupt-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    let persistence = root.join("sync-state.json");
    fs::write(&persistence, b"not-json").expect("corrupt state");
    assert!(MemorySyncStore::open(&persistence).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Rejects baseline blobs whose content no longer matches the manifest digest.
#[test]
fn tampered_persistent_baseline_fails_closed() {
    let root = std::env::temp_dir().join(format!("scriberig-sync-tampered-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    let source = root.join("source.rs");
    let persistence = root.join("sync-state.json");
    fs::write(&source, b"baseline output").expect("source");
    let mut store = MemorySyncStore::open(&persistence).expect("open persistent store");
    store.put(state(&source, b"baseline output"));
    store.persist().expect("persist baseline");
    let bytes = fs::read(&persistence).expect("read state file");
    let encoded: serde_json::Value = serde_json::from_slice(&bytes).expect("state JSON");
    let blob = encoded[0]["baseline_blob"]
        .as_str()
        .expect("baseline content address")
        .to_owned();
    let blob_path = root.join("sync-state.json.blobs").join(blob);
    fs::write(&blob_path, b"tampered baseline").expect("tamper baseline blob");
    fs::write(
        &persistence,
        serde_json::to_vec(&encoded).expect("encode tampered JSON"),
    )
    .expect("write tampered state");

    assert!(MemorySyncStore::open(&persistence).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Removes superseded content blobs after the new manifest has been committed.
#[test]
fn persistent_baseline_replacement_collects_unreferenced_blob() {
    let root = std::env::temp_dir().join(format!("scriberig-sync-gc-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    let source = root.join("source.rs");
    let persistence = root.join("sync-state.json");
    let blob_directory = root.join("sync-state.json.blobs");
    fs::write(&source, b"baseline output").expect("source");
    let mut store = MemorySyncStore::open(&persistence).expect("open persistent store");
    store.put(state(&source, b"baseline output"));
    store.persist().expect("persist first baseline");
    let old_blob = fs::read_dir(&blob_directory)
        .expect("blob directory")
        .next()
        .expect("first blob")
        .expect("read blob entry")
        .path();

    store.put(state(&source, b"replacement output"));
    store.persist().expect("persist replacement baseline");

    assert!(!old_blob.exists());
    let reopened = MemorySyncStore::open(&persistence).expect("reopen replacement");
    assert_eq!(
        reopened
            .find(&ProjectionLocation::new(&source))
            .expect("retained state")
            .baseline
            .rendered
            .bytes,
        b"replacement output"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
