//! Exercises project candidate materialization failure and recovery.
use super::{FileSystem, LocalFileSystem, ProjectService};
use crate::{ProjectSourceChange, ProjectSourceChangeKind};
use sr_projection_fs::{FileMetadata, IoError};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Filesystem that fails one selected atomic write while delegating other IO.
struct FailSecondWrite {
    /// Number of candidate writes observed by the fixture.
    writes: AtomicUsize,
}

impl FileSystem for FailSecondWrite {
    /// Reads exact bytes from the local fixture.
    ///
    /// # Parameters
    /// - `path`: Fixture file to read.
    ///
    /// # Returns
    /// The exact file bytes.
    ///
    /// # Errors
    /// Returns the local filesystem read error.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        LocalFileSystem.read(path)
    }

    /// Reads local fixture metadata.
    ///
    /// # Parameters
    /// - `path`: Fixture path to inspect.
    ///
    /// # Returns
    /// Metadata when the path exists.
    ///
    /// # Errors
    /// Returns the local filesystem metadata error.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError> {
        LocalFileSystem.metadata(path)
    }

    /// Fails the second candidate write and delegates the first.
    ///
    /// # Parameters
    /// - `path`: Fixture file to replace.
    /// - `bytes`: Exact replacement bytes.
    ///
    /// # Returns
    /// Unit after a successful write.
    ///
    /// # Errors
    /// Returns an injected failure on the second write or a local write error.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError> {
        if self.writes.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err(IoError::Write(std::io::Error::other(
                "injected write failure",
            )));
        }
        LocalFileSystem.write_atomic(path, bytes)
    }

    /// Creates directories in the local fixture.
    ///
    /// # Parameters
    /// - `path`: Fixture directory to create.
    ///
    /// # Returns
    /// Unit after directory creation.
    ///
    /// # Errors
    /// Returns the local filesystem directory error.
    fn create_dir_all(&self, path: &Path) -> Result<(), IoError> {
        LocalFileSystem.create_dir_all(path)
    }

    /// Removes files from the local fixture.
    ///
    /// # Parameters
    /// - `path`: Fixture file to remove.
    ///
    /// # Returns
    /// Unit after removal.
    ///
    /// # Errors
    /// Returns the local filesystem removal error.
    fn remove_file(&self, path: &Path) -> Result<(), IoError> {
        LocalFileSystem.remove_file(path)
    }
}

/// Verifies partial materialization is durable and safely retryable.
#[test]
fn partial_effect_failure_is_truthful_and_recoverable() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-candidate-recovery-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"candidate_recovery\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .expect("manifest");
    let base_bytes = b"pub fn base() -> u8 { 1 }\n";
    fs::write(root.join("src/lib.rs"), base_bytes).expect("base source");
    let store = Arc::new(
        scriberig_store_sqlite::SqliteStore::open(root.with_extension(".sqlite"))
            .expect("database"),
    );
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service
        .scan(Some("candidate-recovery".to_owned()), &root)
        .expect("scan");
    let base = service.import(scan.project.id).expect("import").snapshot;
    let candidate = service
        .propose_project_change(
            scan.project.id,
            base,
            vec![
                ProjectSourceChange {
                    path: "src/lib.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Update,
                    content: Some(b"pub fn base() -> u8 { 2 }\n".to_vec()),
                },
                ProjectSourceChange {
                    path: "src/z-generated.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Create,
                    content: Some(b"pub fn generated() {}\n".to_vec()),
                },
            ],
        )
        .expect("aggregate candidate");
    let candidate = service
        .validate_candidate(&candidate.id)
        .expect("isolated validation");
    let filesystem = FailSecondWrite {
        writes: AtomicUsize::new(0),
    };
    let error = service
        .apply_candidate_with_filesystem(&candidate, &filesystem)
        .expect_err("second effect must fail");
    assert_eq!(error.code, "filesystem_write_failed");
    let aggregate = store
        .project_effect(&candidate.id)
        .expect("aggregate receipt")
        .expect("aggregate state");
    assert_eq!(aggregate.state, "unknown");
    assert_eq!(aggregate.successor_snapshot, None);
    let receipts = store
        .project_effect_paths(&candidate.id)
        .expect("per-path receipts");
    assert_eq!(receipts[0].state, "applied");
    assert_eq!(receipts[1].state, "failed");
    assert_eq!(
        service
            .workspace(scan.project.id)
            .expect("base remains")
            .snapshot,
        base
    );
    assert!(
        fs::read_to_string(root.join("src/lib.rs"))
            .expect("first effect bytes")
            .contains("{ 2 }")
    );
    assert!(!root.join("src/z-generated.rs").exists());

    let recovered = service
        .apply_candidate(&candidate)
        .expect("idempotent retry");
    assert_eq!(recovered.effect_state, "applied");
    assert_ne!(recovered.successor_snapshot, base);
    assert!(root.join("src/z-generated.rs").exists());
    assert_eq!(
        store
            .project_effect_paths(&candidate.id)
            .expect("recovered path receipts")
            .iter()
            .filter(|receipt| receipt.state == "applied")
            .count(),
        2
    );
}
