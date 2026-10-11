//! Covers adapter-backed filesystem synchronization cases.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    DiagnosticStage, ProjectionAdapter, ProjectionCapabilities, ProjectionDiagnostic,
    ProjectionKind, ProjectionResult, ReconciliationContext, ReconciliationOutcome, RenderRequest,
    RenderedProjection,
};
use sr_projection::{ProjectionRef, ProjectionStatus, SemanticStateRef};
use sr_projection_fs::{
    FileSystem, LocalFileSystem, MemorySyncStore, Observation, ProjectionLocation,
    ProjectionOwnership, SyncCoordinator, SyncState, digest_bytes,
};
use std::cell::Cell;
use std::fs;

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
    /// The value produced by the declaration.
    fn capabilities(&self, _kind: &ProjectionKind) -> ProjectionCapabilities {
        ProjectionCapabilities::editable()
    }

    /// Implements `render`.
    /// - `request`: Argument named `request`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
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
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
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
    /// The value produced by the declaration.
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
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn validate(&self, _candidate: &Self::Candidate) -> ProjectionResult<()> {
        self.validate_calls.set(self.validate_calls.get() + 1);
        Ok(())
    }
}

/// Creates a synchronization coordinator with one managed projection.
/// - `crate`: Parent test-crate imports used by this helper.
///
/// # Parameters
/// - `path`: Projection file path.
/// - `rendered`: Previously rendered bytes.
///
/// # Returns
/// A coordinator containing one managed editable projection.
pub(crate) fn coordinator(
    path: &std::path::Path,
    rendered: &[u8],
) -> SyncCoordinator<LocalFileSystem> {
    let mut states = MemorySyncStore::default();
    states.put(SyncState {
        projection: ProjectionRef(EntityId::from_bytes([1; 16])),
        specification: sr_projection::ProjectionSpecRef(EntityId::from_bytes([4; 16])),
        scope: sr_projection::SemanticScopeRef {
            entity: EntityId::from_bytes([5; 16]),
            revision: None,
        },
        kind: sr_projection::ProjectionKind::new("text").expect("kind"),
        location: ProjectionLocation::new(path),
        ownership: ProjectionOwnership::ManagedEditable,
        base: SemanticStateRef(SnapshotId::new([2; 32])),
        current: SemanticStateRef(SnapshotId::new([3; 32])),
        style: sr_projection::StyleRef::Value(sr_projection::StyleValue::new(Vec::new())),
        renderer: sr_projection::RendererVersion::new("test"),
        rendered: digest_bytes(rendered),
        observed: None,
        status: ProjectionStatus::Synchronized,
    });
    SyncCoordinator::new(LocalFileSystem, states)
}
