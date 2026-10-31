//! Covers adapter-backed filesystem synchronization cases.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    DiagnosticStage, ProjectionAdapter, ProjectionCapabilities, ProjectionDiagnostic,
    ProjectionKind, ProjectionResult, ReconciliationContext, ReconciliationOutcome, RenderRequest,
    RenderedProjection,
};
use sr_projection::{ProjectionRef, ProjectionStatus, SemanticStateRef};
use sr_projection_fs::{
    FileMetadata, FileSystem, IoError, LocalFileSystem, MemorySyncStore, Observation,
    ProjectionLocation, ProjectionOwnership, SyncCoordinator, SyncState,
};
use std::cell::Cell;
use std::fs;
use std::path::Path;

mod common;

/// Verifies initial managed rendering uses the adapter and synchronization boundary.
#[test]
fn managed_render_uses_adapter_output() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-render-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let mut coordinator = coordinator(&path, b"old");
    let adapter = TestAdapter::default();
    coordinator
        .render_managed_projection(ProjectionRef(EntityId::from_bytes([1; 16])), &adapter)
        .expect("render");
    assert_eq!(LocalFileSystem.read(&path).expect("read"), b"canonical");
    assert_eq!(adapter.render_calls.get(), 1);
    assert_eq!(
        coordinator
            .states()
            .find(&ProjectionLocation::new(&path))
            .unwrap()
            .status,
        ProjectionStatus::Synchronized
    );
    let _ = fs::remove_dir_all(root);
}

/// Verifies the coordinator owns the complete generic adapter lifecycle.
#[test]
fn adapter_import_lifecycle_promotes_and_rerenders() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-adapter-{}", std::process::id()));
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
    let adapter = TestAdapter::default();
    let mut promoted = false;
    coordinator
        .process_import(
            request,
            &adapter,
            |migration, _request| {
                assert_eq!(migration, "migration");
                promoted = true;
                Ok(SemanticStateRef(scriberig_core::SnapshotId::new([9; 32])))
            },
            |()| ProjectionDiagnostic::unsupported(DiagnosticStage::Reconcile),
        )
        .expect("process import");
    assert!(promoted);
    assert_eq!(adapter.parse_calls.get(), 1);
    assert_eq!(adapter.validate_calls.get(), 1);
    assert_eq!(adapter.reconcile_calls.get(), 1);
    assert_eq!(adapter.render_calls.get(), 1);
    assert_eq!(io.read(&path).expect("read"), b"canonical");
    assert_eq!(
        coordinator
            .states()
            .find(&ProjectionLocation::new(&path))
            .unwrap()
            .status,
        ProjectionStatus::Synchronized
    );
    let _ = fs::remove_dir_all(root);
}

/// Verifies ambiguous adapter output retains the external evidence as conflict.
#[test]
fn ambiguous_adapter_result_enters_conflict() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-ambiguous-{}", std::process::id()));
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
    let adapter = TestAdapter {
        ambiguous: true,
        ..TestAdapter::default()
    };
    coordinator
        .process_import(
            request.clone(),
            &adapter,
            |_, _| Ok(SemanticStateRef(scriberig_core::SnapshotId::new([9; 32]))),
            |()| ProjectionDiagnostic::unsupported(DiagnosticStage::Reconcile),
        )
        .expect("process import");
    assert_eq!(
        coordinator.states().find(&request.location).unwrap().status,
        ProjectionStatus::Conflict
    );
    assert_eq!(
        coordinator
            .recoveries()
            .find(request.projection)
            .unwrap()
            .bytes,
        b"external"
    );
    let _ = fs::remove_dir_all(root);
}

/// Verifies a mismatched write leaves the exact prior reconciliation baseline intact.
#[test]
fn mismatched_write_does_not_advance_synchronization_baseline() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-baseline-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let io = AlteredWriteFileSystem(LocalFileSystem);
    io.0.write_atomic(&path, b"rendered").expect("seed file");
    let mut coordinator = coordinator_with_fs(io, &path, b"rendered");
    let prior = coordinator
        .states()
        .find(&ProjectionLocation::new(&path))
        .expect("state")
        .clone();

    assert!(matches!(
        coordinator.write_rendered(
            ProjectionRef(EntityId::from_bytes([1; 16])),
            b"requested",
            SemanticStateRef(SnapshotId::new([8; 32])),
        ),
        Err(IoError::BaselineMismatch)
    ));

    let state = coordinator
        .states()
        .find(&ProjectionLocation::new(&path))
        .expect("state remains available");
    assert_eq!(state.baseline, prior.baseline);
    assert_eq!(state.current, prior.current);
    assert_eq!(state.status, ProjectionStatus::ExternallyModified);
    assert_eq!(
        io.0.read(&path).expect("read altered output"),
        b"requested!"
    );
    let _ = fs::remove_dir_all(root);
}

#[derive(Clone, Copy)]
/// Simulates a successful atomic write that stores bytes different from its input.
struct AlteredWriteFileSystem(LocalFileSystem);

impl FileSystem for AlteredWriteFileSystem {
    /// Reads bytes from the local filesystem.
    ///
    /// - `self`: Altered-write filesystem.
    /// - `path`: Path to read.
    ///
    /// The exact file bytes.
    ///
    /// # Returns
    /// The bytes when the read succeeds.
    ///
    /// # Errors
    /// Returns the local filesystem read error.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        self.0.read(path)
    }

    /// Reads metadata from the local filesystem.
    ///
    /// - `self`: Altered-write filesystem.
    /// - `path`: Path to inspect.
    ///
    /// Metadata when the path exists.
    ///
    /// # Returns
    /// The optional file metadata.
    ///
    /// # Errors
    /// Returns the local filesystem metadata error.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError> {
        self.0.metadata(path)
    }

    /// Stores a deliberately altered byte sequence to exercise read-back validation.
    ///
    /// - `self`: Altered-write filesystem.
    /// - `path`: Target file path.
    /// - `bytes`: Requested output bytes.
    ///
    /// Unit after the altered bytes are stored.
    ///
    /// # Returns
    /// Unit after the local atomic write succeeds.
    ///
    /// # Errors
    /// Returns the local filesystem write error.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError> {
        let mut altered = bytes.to_vec();
        altered.push(b'!');
        self.0.write_atomic(path, &altered)
    }

    /// Creates directories through the local filesystem.
    ///
    /// - `self`: Altered-write filesystem.
    /// - `path`: Directory path to create.
    ///
    /// Unit after directory creation.
    ///
    /// # Returns
    /// Unit after the local operation succeeds.
    ///
    /// # Errors
    /// Returns the local filesystem directory error.
    fn create_dir_all(&self, path: &Path) -> Result<(), IoError> {
        self.0.create_dir_all(path)
    }

    /// Removes a file through the local filesystem.
    ///
    /// - `self`: Altered-write filesystem.
    /// - `path`: File path to remove.
    ///
    /// Unit after removal.
    ///
    /// # Returns
    /// Unit after the local operation succeeds.
    ///
    /// # Errors
    /// Returns the local filesystem removal error.
    fn remove_file(&self, path: &Path) -> Result<(), IoError> {
        self.0.remove_file(path)
    }
}

#[derive(Default)]
/// Provides a deterministic projection adapter for conformance tests.
struct TestAdapter {
    ambiguous: bool,
    parse_calls: Cell<u32>,
    validate_calls: Cell<u32>,
    reconcile_calls: Cell<u32>,
    render_calls: Cell<u32>,
}

impl ProjectionAdapter for TestAdapter {
    type Candidate = bool;
    type Migration = &'static str;
    type Conflict = ();

    /// Implements `capabilities`.
    /// - `_kind`: Argument named `_kind`.
    ///
    /// # Returns
    /// The operation result.
    fn capabilities(&self, _kind: &ProjectionKind) -> ProjectionCapabilities {
        ProjectionCapabilities::editable()
    }

    /// Implements `render`.
    /// - `request`: Argument named `request`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection> {
        self.render_calls.set(self.render_calls.get() + 1);
        Ok(RenderedProjection::from_bytes(
            b"canonical".to_vec(),
            request,
            None,
        ))
    }

    /// Implements `parse`.
    /// - `_kind`: Argument named `_kind`.
    /// - `bytes`: Argument named `bytes`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn parse(&self, _kind: &ProjectionKind, bytes: &[u8]) -> ProjectionResult<Self::Candidate> {
        self.parse_calls.set(self.parse_calls.get() + 1);
        if bytes == b"external" {
            Ok(true)
        } else {
            Err(Box::new(ProjectionDiagnostic::unsupported(
                DiagnosticStage::Parse,
            )))
        }
    }

    /// Implements `reconcile`.
    /// - `_candidate`: Argument named `_candidate`.
    /// - `_context`: Argument named `_context`.
    ///
    /// # Returns
    /// The operation result.
    fn reconcile(
        &self,
        _candidate: Self::Candidate,
        _context: &ReconciliationContext,
    ) -> ReconciliationOutcome<Self::Migration, Self::Conflict, Box<ProjectionDiagnostic>> {
        self.reconcile_calls.set(self.reconcile_calls.get() + 1);
        if self.ambiguous {
            return ReconciliationOutcome::Ambiguous(Box::new(ProjectionDiagnostic::unsupported(
                DiagnosticStage::Reconcile,
            )));
        }
        ReconciliationOutcome::MigrationProposal("migration")
    }

    /// Implements `validate`.
    /// - `_candidate`: Argument named `_candidate`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn validate(&self, _candidate: &Self::Candidate) -> ProjectionResult<()> {
        self.validate_calls.set(self.validate_calls.get() + 1);
        Ok(())
    }
}

/// Creates a synchronization coordinator with one managed projection.
/// - `crate`: Parent test-crate imports used by this helper.
///
/// - `path`: Projection file path.
/// - `rendered`: Previously rendered bytes.
///
/// A coordinator containing one managed editable projection.
/// # Returns
/// The operation result.
pub(crate) fn coordinator(
    path: &std::path::Path,
    rendered: &[u8],
) -> SyncCoordinator<LocalFileSystem> {
    coordinator_with_fs(LocalFileSystem, path, rendered)
}

/// Creates a synchronization coordinator with one managed projection and caller-supplied IO.
///
/// - `filesystem`: Filesystem implementation used by the coordinator.
/// - `path`: Projection file path.
/// - `rendered`: Previously rendered bytes.
///
/// A coordinator containing one managed editable projection.
fn coordinator_with_fs<F: FileSystem>(
    filesystem: F,
    path: &std::path::Path,
    rendered: &[u8],
) -> SyncCoordinator<F> {
    let mut states = MemorySyncStore::default();
    let mut state = SyncState::new(
        ProjectionRef(EntityId::from_bytes([1; 16])),
        common::baseline("text", SnapshotId::new([2; 32]), &[], rendered),
        ProjectionLocation::new(path),
        ProjectionOwnership::ManagedEditable,
        SemanticStateRef(SnapshotId::new([3; 32])),
        ProjectionStatus::Synchronized,
    );
    state.baseline.rendered.cache_key.scope.entity = EntityId::from_bytes([5; 16]);
    state.baseline.rendered.cache_key.specification =
        sr_projection::ProjectionSpecRef(EntityId::from_bytes([4; 16]));
    states.put(state);
    SyncCoordinator::new(filesystem, states)
}
