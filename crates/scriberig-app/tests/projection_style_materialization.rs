//! Proves a shared-purpose resolved style is what reaches materialized files.

#[allow(dead_code)] // This binary uses a subset of the shared style fixtures.
#[path = "projection_style_fixture.rs"]
mod fixture;

use std::cell::Cell;
use std::sync::Arc;

use scriberig_app::{
    ApplicationService, ProjectionStyleBackend, ProjectionStyleConfiguration,
    ProjectionStyleSettingRequest,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_style_sqlite::SqliteProjectionStyleStore;
use sr_projection::{
    ProjectionAdapter, ProjectionCapabilities, ProjectionKind, ProjectionPurpose, ProjectionResult,
    RenderRequest, RenderedProjection, StyleRef, StyleValue,
};
use sr_projection_fs::{
    FileSystem as _, LocalFileSystem, MemorySyncStore, ProjectionLocation, ProjectionOwnership,
    SyncCoordinator, SyncState, digest_bytes,
};

use fixture::{DocumentStyleAdapter, RecordingBackend, context_for, kind, style};

/// Adapter that writes the exact style bytes into the file, so the materialized
/// bytes show precisely which resolved style was applied.
///
/// # Public members
/// - `calls`: Number of renders performed.
///
/// # Private members
/// - None.
#[derive(Default)]
struct StyleEchoAdapter {
    /// Number of renders performed.
    calls: Cell<u32>,
}

impl ProjectionAdapter for StyleEchoAdapter {
    type Candidate = bool;
    type Migration = &'static str;
    type Conflict = ();

    /// Declares the representation editable.
    ///
    /// - `_kind`: Argument named `_kind`.
    ///
    /// # Returns
    /// The operation result.
    fn capabilities(&self, _kind: &ProjectionKind) -> ProjectionCapabilities {
        ProjectionCapabilities::editable()
    }

    /// Renders the exact style bytes as the file body.
    ///
    /// - `self`: Adapter performing rendering.
    /// - `request`: Exact render inputs.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns an error when the operation fails.
    fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection> {
        self.calls.set(self.calls.get() + 1);
        Ok(RenderedProjection::from_bytes(
            request.style.value().as_bytes().to_vec(),
            request,
            None,
        ))
    }
}

/// Builds one service whose style settings live in the durable store at `path`.
///
/// - `path`: SQLite database path holding the style settings.
///
/// # Returns
/// The service and a backend probe counting canonical requests.
fn service_on(path: &std::path::Path) -> (ApplicationService<RecordingBackend>, RecordingBackend) {
    let store =
        Arc::new(SqliteProjectionStyleStore::open(path).expect("durable style store opens"));
    let service =
        ApplicationService::new(RecordingBackend::default()).with_projection_style_configuration(
            ProjectionStyleConfiguration::new(Arc::new(DocumentStyleAdapter), store),
        );
    (service, RecordingBackend::default())
}

/// Builds one filesystem coordinator managing one projection at `path`.
///
/// - `path`: Managed file location.
///
/// # Returns
/// The coordinator with its synchronization state.
fn coordinator(path: &std::path::Path) -> SyncCoordinator<LocalFileSystem> {
    let mut states = MemorySyncStore::default();
    states.put(SyncState {
        projection: sr_projection::ProjectionRef(EntityId::from_bytes([1; 16])),
        specification: sr_projection::ProjectionSpecRef(EntityId::from_bytes([4; 16])),
        scope: sr_projection::SemanticScopeRef {
            entity: EntityId::from_bytes([5; 16]),
            revision: None,
        },
        kind: kind(),
        location: ProjectionLocation::new(path),
        ownership: ProjectionOwnership::ManagedEditable,
        base: sr_projection::SemanticStateRef(SnapshotId::new([2; 32])),
        current: sr_projection::SemanticStateRef(SnapshotId::new([3; 32])),
        style: StyleRef::Value(StyleValue::new(Vec::new())),
        renderer: sr_projection::RendererVersion::new("test"),
        rendered: digest_bytes(b"seed"),
        observed: None,
        status: sr_projection::ProjectionStatus::Synchronized,
    });
    SyncCoordinator::new(LocalFileSystem, states)
}

/// Verifies materialization applies the shared purpose style, never the personal
/// preference stored for the same principal.
#[test]
fn materialization_writes_the_shared_style_and_not_the_personal_preference() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("scriberig.db");
    let file = directory.path().join("document.txt");

    let (writer, _probe) = service_on(&database);
    writer
        .set_projection_style_user_preference(
            &context_for("alice"),
            &fixture::set_style(&[("width", "100")]),
        )
        .expect("personal preference written");
    let policy =
        ProjectionStyleSettingRequest::new(kind(), "ws-1", Some(style(&[("width", "80")])));
    writer
        .set_projection_style_materialization_policy(&context_for("alice"), &policy)
        .expect("workspace materialization policy written");

    let (reader, _probe) = service_on(&database);
    let request = scriberig_app::StyleResolutionRequest::new(
        kind(),
        "proj-1",
        "ws-1",
        ProjectionPurpose::SharedMaterialization,
    );
    let effective = reader
        .projection_style_effective(&context_for("alice"), &request)
        .expect("shared style resolves");

    let mut coordinator = coordinator(&file);
    let adapter = StyleEchoAdapter::default();
    coordinator
        .render_managed_projection_with_style(
            sr_projection::ProjectionRef(EntityId::from_bytes([1; 16])),
            &adapter,
            &effective.resolved,
        )
        .expect("materialization render succeeds");

    let written = LocalFileSystem
        .read(&file)
        .expect("materialized file readable");
    assert!(
        String::from_utf8_lossy(&written).contains("\"width\":\"80\""),
        "materialized bytes must carry the workspace policy: {written:?}"
    );
    assert!(
        !String::from_utf8_lossy(&written).contains("100"),
        "a personal preference must never reach a materialized file: {written:?}"
    );
    assert_eq!(written, effective.resolved.value().as_bytes());
}
