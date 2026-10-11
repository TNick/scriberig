//! Verifies exact-byte filesystem projection behavior.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    DiagnosticStage, ProjectionAdapter, ProjectionCapabilities, ProjectionDiagnostic,
    ProjectionKind, ProjectionResult, ReconciliationContext, ReconciliationOutcome, RenderRequest,
    RenderedProjection,
};
use sr_projection::{ProjectionRef, ProjectionStatus, SemanticStateRef};
use sr_projection_fs::{
    FileSystem, ImportCompletion, LocalFileSystem, MemorySyncStore, Observation,
    ProjectionLocation, ProjectionOwnership, SyncCoordinator, SyncState, digest_bytes,
};
use std::cell::Cell;
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
        ProjectionLocation::new("folder\\.\\nested\\..\\file.txt"),
        ProjectionLocation::new("folder\\file.txt")
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
    let projection = ProjectionRef(EntityId::new([1; 16]));
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

/// Verifies initial managed rendering uses the adapter and synchronization boundary.
#[test]
fn managed_render_uses_adapter_output() {
    let root = std::env::temp_dir().join(format!("scriberig-fs-render-{}", std::process::id()));
    let _ = fs::create_dir_all(&root);
    let path = root.join("projection.txt");
    let mut coordinator = coordinator(&path, b"old");
    let adapter = TestAdapter::default();
    coordinator
        .render_managed_projection(ProjectionRef(EntityId::new([1; 16])), &adapter)
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

    fn capabilities(&self, _kind: &ProjectionKind) -> ProjectionCapabilities {
        ProjectionCapabilities::editable()
    }

    fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection> {
        self.render_calls.set(self.render_calls.get() + 1);
        Ok(RenderedProjection::from_bytes(
            b"canonical".to_vec(),
            request,
            None,
        ))
    }

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

    fn validate(&self, _candidate: &Self::Candidate) -> ProjectionResult<()> {
        self.validate_calls.set(self.validate_calls.get() + 1);
        Ok(())
    }
}

/// Creates a synchronization coordinator with one managed projection.
///
/// # Parameters
/// - `path`: Projection file path.
/// - `rendered`: Previously rendered bytes.
///
/// # Returns
/// A coordinator containing one managed editable projection.
fn coordinator(path: &std::path::Path, rendered: &[u8]) -> SyncCoordinator<LocalFileSystem> {
    let mut states = MemorySyncStore::default();
    states.put(SyncState {
        projection: ProjectionRef(EntityId::new([1; 16])),
        specification: sr_projection::ProjectionSpecRef(EntityId::new([4; 16])),
        scope: sr_projection::SemanticScopeRef {
            entity: EntityId::new([5; 16]),
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
