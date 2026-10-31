//! Exercises project candidate materialization failure and recovery.
use super::{FileSystem, LocalFileSystem, ProjectService};
use crate::{
    ProjectCandidate, ProjectSourceChange, ProjectSourceChangeKind, ProjectSourceObservation,
};
use scriberig_core::{ProjectId, SnapshotId};
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
    /// - `path`: Fixture file to read.
    ///
    /// The exact file bytes.
    ///
    /// Returns the local filesystem read error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        LocalFileSystem.read(path)
    }

    /// Reads local fixture metadata.
    ///
    /// - `path`: Fixture path to inspect.
    ///
    /// Metadata when the path exists.
    ///
    /// Returns the local filesystem metadata error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError> {
        LocalFileSystem.metadata(path)
    }

    /// Fails the second candidate write and delegates the first.
    ///
    /// - `path`: Fixture file to replace.
    /// - `bytes`: Exact replacement bytes.
    ///
    /// Unit after a successful write.
    ///
    /// Returns an injected failure on the second write or a local write error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `path`: Fixture directory to create.
    ///
    /// Unit after directory creation.
    ///
    /// Returns the local filesystem directory error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_dir_all(&self, path: &Path) -> Result<(), IoError> {
        LocalFileSystem.create_dir_all(path)
    }

    /// Removes files from the local fixture.
    ///
    /// - `path`: Fixture file to remove.
    ///
    /// Unit after removal.
    ///
    /// Returns the local filesystem removal error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn remove_file(&self, path: &Path) -> Result<(), IoError> {
        LocalFileSystem.remove_file(path)
    }
}

/// Verifies partial materialization remains recoverable across a store restart.
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
    let database = root.with_extension(".sqlite");
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(&database).expect("database"));
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
    assert_partial_failure_state(
        &store,
        &service,
        &candidate,
        base,
        scan.project.id,
        &root,
        &error,
    );

    drop(service);
    drop(store);
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(&database).expect("reopen db"));
    let service = ProjectService::new(Arc::clone(&store));
    assert_eq!(
        service
            .workspace(scan.project.id)
            .expect("workspace")
            .snapshot,
        base
    );
    let recovered = service
        .apply_candidate(&candidate)
        .expect("idempotent retry");
    assert_eq!(recovered.effect_state, "applied");
    assert_ne!(recovered.successor_snapshot, base);
    assert!(root.join("src/z-generated.rs").exists());
    assert_source_watch_receipt(&service, scan.project.id, &root);
    assert_eq!(
        store
            .project_effect_paths(&candidate.id)
            .expect("recovered path receipts")
            .iter()
            .filter(|receipt| receipt.state == "applied")
            .count(),
        2
    );
    assert_successor_artifact_digests(&service, scan.project.id, &root);
}

/// Verifies candidate receipts suppress matching watcher events only.
///
/// # Parameters
/// - `service`: Project service that applied and recorded candidate effects.
/// - `project`: Exact project identity for the observed source path.
/// - `root`: Project checkout containing the committed source bytes.
fn assert_source_watch_receipt(service: &ProjectService, project: ProjectId, root: &Path) {
    let committed_bytes = fs::read(root.join("src/lib.rs")).expect("committed source bytes");
    let committed_digest = *blake3::hash(&committed_bytes).as_bytes();
    assert_eq!(
        service
            .classify_source_observation(project, "src/lib.rs", Some(committed_digest))
            .expect("classify committed write"),
        ProjectSourceObservation::SelfWrite
    );
    assert_eq!(
        service
            .classify_source_observation(project, "src/lib.rs", Some(committed_digest))
            .expect("duplicate watcher event"),
        ProjectSourceObservation::SelfWrite
    );
    assert_eq!(
        service
            .classify_source_observation(project, "src/lib.rs", Some([7; 32]))
            .expect("external modification event"),
        ProjectSourceObservation::ExternalChange
    );
}

/// Verifies the durable partial state before and after the injected write failure.
///
/// # Parameters
/// - `store`: Durable store containing effect receipts.
/// - `service`: Project service still at the exact base snapshot.
/// - `candidate`: Candidate whose first member was committed.
/// - `base`: Original project snapshot.
/// - `project`: Exact project identity.
/// - `root`: Project checkout containing the partial materialization.
/// - `error`: Failed apply result.
fn assert_partial_failure_state(
    store: &scriberig_store_sqlite::SqliteStore,
    service: &ProjectService,
    candidate: &ProjectCandidate,
    base: SnapshotId,
    project: ProjectId,
    root: &Path,
    error: &crate::ProjectError,
) {
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
    let public_receipt = service
        .materialization_receipt(&candidate.id)
        .expect("public recovery receipt")
        .expect("receipt after partial materialization");
    assert_eq!(public_receipt.candidate, candidate.id);
    assert_eq!(public_receipt.state, "unknown");
    assert_eq!(public_receipt.paths[0].state, "applied");
    assert_eq!(public_receipt.paths[1].state, "failed");
    assert!(
        public_receipt.paths[1]
            .diagnostic
            .as_deref()
            .is_some_and(|message| !message.is_empty())
    );
    assert_eq!(
        service.workspace(project).expect("base remains").snapshot,
        base
    );
    assert!(
        fs::read_to_string(root.join("src/lib.rs"))
            .expect("first effect bytes")
            .contains("{ 2 }")
    );
    assert!(!root.join("src/z-generated.rs").exists());
}

/// Verifies refreshed artifact records use the exact committed source bytes.
///
/// # Parameters
/// - `service`: Reopened project service with the recovered successor.
/// - `project`: Project whose artifact summaries are checked.
/// - `root`: Materialized project checkout.
fn assert_successor_artifact_digests(service: &ProjectService, project: ProjectId, root: &Path) {
    let artifacts = service
        .artifacts(project)
        .expect("successor artifact baseline");
    for path in ["src/lib.rs", "src/z-generated.rs"] {
        let expected =
            *blake3::hash(&fs::read(root.join(path)).expect("materialized bytes")).as_bytes();
        assert!(artifacts.iter().any(|artifact| {
            artifact.members.iter().any(|member| {
                member.project_path.as_deref() == Some(path) && member.digest == expected
            })
        }));
    }
}
