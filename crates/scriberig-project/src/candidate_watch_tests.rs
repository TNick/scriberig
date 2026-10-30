//! Verifies watcher notifications wait for candidate materialization receipts.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use super::{FileSystem, LocalFileSystem};
use crate::{
    ProjectService, ProjectSourceChange, ProjectSourceChangeKind, ProjectSourceObservation,
};
use scriberig_store_sqlite::SqliteStore;
use sr_projection_fs::{FileMetadata, IoError};

/// Pauses the first materialized write until a watcher notification is queued.
struct PauseFirstWrite {
    /// Signals that candidate materialization owns the watcher coordinator.
    started: Sender<()>,
    /// Releases the blocked write after the watcher attempts classification.
    release: Receiver<()>,
    /// Ensures only the first write is paused.
    paused: AtomicBool,
}

impl FileSystem for PauseFirstWrite {
    /// Reads exact bytes from the local fixture.
    ///
    /// - `path`: Exact project source path to read.
    ///
    /// The file's bytes.
    ///
    /// # Returns
    /// The exact file contents.
    ///
    /// # Errors
    /// Returns an error when the local read fails.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        LocalFileSystem.read(path)
    }

    /// Reads local fixture metadata.
    ///
    /// - `path`: Exact project path to inspect.
    ///
    /// Optional file metadata.
    ///
    /// # Returns
    /// The metadata when the path exists.
    ///
    /// # Errors
    /// Returns an error when metadata lookup fails.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError> {
        LocalFileSystem.metadata(path)
    }

    /// Pauses the first replacement, then atomically writes fixture bytes.
    ///
    /// - `path`: Target file path.
    /// - `bytes`: Exact replacement bytes.
    ///
    /// Unit after a successful write.
    ///
    /// # Returns
    /// Unit after the replacement succeeds.
    ///
    /// # Errors
    /// Returns an error when test synchronization or the atomic write fails.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError> {
        if !self.paused.swap(true, Ordering::SeqCst) {
            self.started
                .send(())
                .map_err(|error| IoError::Write(std::io::Error::other(error.to_string())))?;
            self.release
                .recv()
                .map_err(|error| IoError::Write(std::io::Error::other(error.to_string())))?;
        }
        LocalFileSystem.write_atomic(path, bytes)
    }

    /// Creates a local fixture directory tree.
    ///
    /// - `path`: Directory path to create.
    ///
    /// Unit after directory creation.
    ///
    /// # Returns
    /// Unit after the directory exists.
    ///
    /// # Errors
    /// Returns an error when directory creation fails.
    fn create_dir_all(&self, path: &Path) -> Result<(), IoError> {
        LocalFileSystem.create_dir_all(path)
    }

    /// Removes a local fixture file.
    ///
    /// - `path`: File path to remove.
    ///
    /// Unit after removal.
    ///
    /// # Returns
    /// Unit after the file is removed.
    ///
    /// # Errors
    /// Returns an error when removal fails.
    fn remove_file(&self, path: &Path) -> Result<(), IoError> {
        LocalFileSystem.remove_file(path)
    }
}

/// Suppresses a self-write event queued while candidate bytes are being written.
#[test]
fn watcher_event_waits_for_candidate_receipt_before_classification() {
    let root =
        std::env::temp_dir().join(format!("scriberig-watcher-receipt-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"watcher_receipt\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .expect("manifest");
    let original = b"pub fn base() -> u8 { 1 }\n";
    let replacement = b"pub fn base() -> u8 { 2 }\n";
    fs::write(root.join("src/lib.rs"), original).expect("base source");
    let store =
        Arc::new(SqliteStore::open(root.with_extension("sqlite")).expect("open project store"));
    let service = ProjectService::new(store);
    let scan = service
        .scan(Some("watcher-receipt".to_owned()), &root)
        .expect("scan project");
    let base = service
        .import(scan.project.id)
        .expect("import base")
        .snapshot;
    let candidate = service
        .propose_project_change(
            scan.project.id,
            base,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(replacement.to_vec()),
            }],
        )
        .expect("candidate");
    let candidate = service.validate_candidate(&candidate.id).expect("validate");
    let (write_started_tx, write_started_rx) = channel();
    let (release_write_tx, release_write_rx) = channel();
    let filesystem = PauseFirstWrite {
        started: write_started_tx,
        release: release_write_rx,
        paused: AtomicBool::new(false),
    };
    let apply_service = service.clone();
    let apply = std::thread::spawn(move || {
        apply_service.apply_candidate_with_filesystem(&candidate, &filesystem)
    });
    write_started_rx
        .recv()
        .expect("candidate write reached filesystem");

    let (watch_started_tx, watch_started_rx) = channel();
    let (watch_result_tx, watch_result_rx) = channel();
    let watch_service = service.clone();
    let digest = *blake3::hash(replacement).as_bytes();
    let watcher = std::thread::spawn(move || {
        watch_started_tx.send(()).expect("notify watcher start");
        let result =
            watch_service.handle_source_observation(scan.project.id, "src/lib.rs", Some(digest));
        watch_result_tx.send(result).expect("send watcher result");
    });
    watch_started_rx.recv().expect("watcher attempted event");
    assert!(
        watch_result_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err()
    );

    release_write_tx.send(()).expect("release candidate write");
    let report = apply
        .join()
        .expect("apply thread")
        .expect("candidate apply");
    let observation = watch_result_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("watcher resumes after candidate receipt")
        .expect("watcher result");
    watcher.join().expect("watcher thread");
    assert_eq!(observation.observation, ProjectSourceObservation::SelfWrite);
    assert_eq!(observation.imported_snapshot, None);
    assert_eq!(
        service
            .workspace(scan.project.id)
            .expect("workspace")
            .snapshot,
        report.successor_snapshot
    );
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("materialized source"),
        replacement
    );
}
