//! Verifies exact-byte filesystem projection behavior.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{ProjectionRef, ProjectionStatus, SemanticStateRef};
use sr_projection_fs::{
    FileSystem, ImportCompletion, LocalFileSystem, Observation, ProjectionLocation,
    ProjectionOwnership, digest_bytes,
};
use std::fs;

/// Verifies atomic replacement and exact digest changes.
#[test]
fn atomic_write_preserves_exact_bytes() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let io = LocalFileSystem;
    io.write_atomic(&path, b"one\n").expect("write");
    assert_eq!(io.read(&path).expect("read"), b"one\n");
    let first = digest_bytes(b"one\n");
    io.write_atomic(&path, b"one").expect("replace");
    assert_eq!(io.read(&path).expect("read"), b"one");
    assert_ne!(first, digest_bytes(b"one"));
    let _ = fs::remove_dir_all(root);
}

/// Verifies ownership and location remain independent from semantic identity.
#[test]
fn ownership_denies_unmanaged_writes() {
    assert!(!ProjectionOwnership::Unmanaged.may_write());
    assert!(ProjectionOwnership::Unmanaged.forbids_automatic_write());
    assert_ne!(
        ProjectionLocation::new("a.txt"),
        ProjectionLocation::new("b.txt")
    );
    assert_eq!(
        ProjectionLocation::new("folder/./nested/../file.txt"),
        ProjectionLocation::new("folder/file.txt")
    );
}

/// Verifies read-only projections surface drift without scheduling import.
#[test]
fn read_only_drift_does_not_schedule_import() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-read-only-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let io = LocalFileSystem;
    io.write_atomic(&path, b"external").expect("write");
    let location = ProjectionLocation::new(&path);
    let mut coordinator = coordinator(&path, b"rendered");
    coordinator
        .states_mut()
        .find_mut(&location)
        .expect("state")
        .ownership = ProjectionOwnership::ManagedReadOnly;

    let result = coordinator
        .observe(&Observation::Modified(location))
        .expect("observe")
        .expect("known projection");
    assert_eq!(result.status, ProjectionStatus::ExternallyModified);
    assert!(result.import.is_none());
    let _ = fs::remove_dir_all(root);
}

/// Verifies duplicate watcher hints produce one stable import request.
#[test]
fn repeated_external_events_are_idempotent() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-repeat-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let io = LocalFileSystem;
    io.write_atomic(&path, b"external").expect("write");
    let location = ProjectionLocation::new(&path);
    let mut coordinator = coordinator(&path, b"rendered");

    let first = coordinator
        .observe(&Observation::Modified(location.clone()))
        .expect("observe")
        .expect("known projection");
    let second = coordinator
        .observe(&Observation::Modified(location))
        .expect("observe")
        .expect("known projection");

    assert_eq!(first.status, ProjectionStatus::ImportPending);
    assert_eq!(first.import, second.import);
    assert_eq!(first.import.expect("import request").bytes, b"external");
    let _ = fs::remove_dir_all(root);
}

/// Verifies failed imports retain the exact external bytes and base state.
#[test]
fn failed_import_retains_three_way_evidence() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-failure-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let io = LocalFileSystem;
    io.write_atomic(&path, b"invalid").expect("write");
    let location = ProjectionLocation::new(&path);
    let mut coordinator = coordinator(&path, b"rendered");
    let request = coordinator
        .observe(&Observation::Modified(location))
        .expect("observe")
        .expect("known projection")
        .import
        .expect("import request");

    coordinator
        .complete_import(
            request.projection,
            ImportCompletion::failed(request.clone(), "parse failed"),
        )
        .expect("record failure");
    let recovery = coordinator
        .recoveries()
        .find(request.projection)
        .expect("recovery record");
    assert_eq!(recovery.base, SemanticStateRef(SnapshotId::new([2; 32])));
    assert_eq!(recovery.bytes, b"invalid");
    assert_eq!(
        coordinator
            .states()
            .find(&ProjectionLocation::new(&path))
            .unwrap()
            .status,
        ProjectionStatus::ImportFailed
    );
    assert_eq!(
        coordinator.retry_request(request.projection).unwrap().bytes,
        b"invalid"
    );
    let _ = fs::remove_dir_all(root);
}

/// Verifies canonical and external states remain distinct import inputs.
#[test]
fn import_request_exposes_base_current_and_external() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-three-way-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let io = LocalFileSystem;
    io.write_atomic(&path, b"external").expect("write");
    let location = ProjectionLocation::new(&path);
    let mut coordinator = coordinator(&path, b"rendered");
    coordinator
        .states_mut()
        .find_mut(&location)
        .expect("state")
        .current = SemanticStateRef(SnapshotId::new([5; 32]));

    let request = coordinator
        .observe(&Observation::Modified(location))
        .expect("observe")
        .expect("known projection")
        .import
        .expect("import request");
    assert_eq!(request.base, SemanticStateRef(SnapshotId::new([2; 32])));
    assert_eq!(request.current, SemanticStateRef(SnapshotId::new([5; 32])));
    assert_eq!(request.bytes, b"external");
    let _ = fs::remove_dir_all(root);
}

/// Verifies successful rerender updates metadata only after the file exists.
#[test]
fn successful_rerender_establishes_a_new_base() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-rerender-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let mut coordinator = coordinator(&path, b"rendered");
    let projection = ProjectionRef(EntityId::from_bytes([1; 16]));
    coordinator
        .write_rendered(
            projection,
            b"canonical",
            SemanticStateRef(SnapshotId::new([6; 32])),
        )
        .expect("render");
    let state = coordinator
        .states()
        .find(&ProjectionLocation::new(&path))
        .expect("state");
    assert_eq!(state.status, ProjectionStatus::Synchronized);
    assert_eq!(state.base, SemanticStateRef(SnapshotId::new([6; 32])));
    assert_eq!(LocalFileSystem.read(&path).expect("read"), b"canonical");
    let _ = fs::remove_dir_all(root);
}

/// Verifies a successful domain completion writes and promotes its new base.
#[test]
fn completed_import_resynchronizes_after_domain_acceptance() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-complete-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let io = LocalFileSystem;
    io.write_atomic(&path, b"external").expect("write");
    let location = ProjectionLocation::new(&path);
    let mut coordinator = coordinator(&path, b"rendered");
    let request = coordinator
        .observe(&Observation::Modified(location))
        .expect("observe")
        .expect("known projection")
        .import
        .expect("import request");
    let projection = request.projection;
    coordinator
        .complete_import(
            projection,
            ImportCompletion::NoSemanticChange {
                request,
                bytes: b"canonical".to_vec(),
                base: SemanticStateRef(SnapshotId::new([7; 32])),
            },
        )
        .expect("complete import");
    assert_eq!(
        coordinator
            .states()
            .find(&ProjectionLocation::new(&path))
            .unwrap()
            .status,
        ProjectionStatus::Synchronized
    );
    assert_eq!(io.read(&path).expect("read"), b"canonical");
    let _ = fs::remove_dir_all(root);
}

#[path = "sync_conformance_tail.rs"]
mod sync_conformance_tail;
use sync_conformance_tail::coordinator;
