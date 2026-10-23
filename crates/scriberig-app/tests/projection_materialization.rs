//! Proves the product materialization operation writes real files under the
//! workspace materialization style and never under a stored personal preference.

#[allow(dead_code)] // This binary uses a subset of the shared style fixtures.
#[path = "projection_style_fixture.rs"]
mod fixture;

use std::cell::Cell;

use scriberig_app::{
    ApplicationError, ApplicationService, ProjectionMaterializationRequest, ProjectionStyleBackend,
    ProjectionStyleSettingRequest,
};
use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    ProjectionAdapter, ProjectionCapabilities, ProjectionKind, ProjectionRef, ProjectionResult,
    RenderRequest, RenderedProjection, StyleRef, StyleSourceKind, StyleValue,
};
use sr_projection_fs::{
    FileSystem as _, LocalFileSystem, MemorySyncStore, ProjectionLocation, ProjectionOwnership,
    SyncCoordinator, SyncState, digest_bytes,
};

use fixture::{RecordingBackend, context_for, fields, kind, service};

/// Adapter that writes the exact resolved style bytes as the file body, so the
/// materialized bytes show precisely which style the application applied.
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

/// Names the one managed projection materialized by a case in this binary.
///
/// # Returns
/// The projection reference used by the coordinator fixture.
fn projection() -> ProjectionRef {
    ProjectionRef(EntityId::from_bytes([11; 16]))
}

/// Builds one filesystem coordinator managing `path` under `ownership`.
///
/// - `path`: Managed file location.
/// - `ownership`: Filesystem ownership class for that location.
///
/// # Returns
/// The coordinator with its synchronization state.
fn coordinator(
    path: &std::path::Path,
    ownership: ProjectionOwnership,
) -> SyncCoordinator<LocalFileSystem> {
    let mut states = MemorySyncStore::default();
    states.put(SyncState {
        projection: projection(),
        specification: sr_projection::ProjectionSpecRef(EntityId::from_bytes([12; 16])),
        scope: sr_projection::SemanticScopeRef {
            entity: EntityId::from_bytes([13; 16]),
            revision: None,
        },
        kind: kind(),
        location: ProjectionLocation::new(path),
        ownership,
        base: sr_projection::SemanticStateRef(SnapshotId::new([14; 32])),
        current: sr_projection::SemanticStateRef(SnapshotId::new([15; 32])),
        style: StyleRef::Value(StyleValue::new(Vec::new())),
        renderer: sr_projection::RendererVersion::new("test"),
        rendered: digest_bytes(b"seed"),
        observed: None,
        status: sr_projection::ProjectionStatus::Synchronized,
    });
    SyncCoordinator::new(LocalFileSystem, states)
}

/// Stores one workspace materialization policy through the application surface.
///
/// - `service`: Application service owning the style configuration.
/// - `pairs`: Style fields the workspace policy names.
///
/// # Panics
/// Panics when the policy write fails.
fn store_workspace_policy(service: &ApplicationService<RecordingBackend>, pairs: &[(&str, &str)]) {
    let request = ProjectionStyleSettingRequest::new(kind(), "ws-1", Some(fixture::style(pairs)));
    service
        .set_projection_style_materialization_policy(&context_for("alice"), &request)
        .expect("workspace materialization policy stored");
}

/// Asks the application to materialize the fixture projection for `ws-1`.
///
/// - `service`: Application service resolving the shared style.
/// - `coordinator`: Filesystem boundary holding the projection state.
/// - `adapter`: Rendering adapter echoing the applied style into the file.
///
/// # Returns
/// The materialization receipt.
///
/// # Panics
/// Panics when materialization fails.
fn materialize(
    service: &ApplicationService<RecordingBackend>,
    coordinator: &mut SyncCoordinator<LocalFileSystem>,
    adapter: &StyleEchoAdapter,
) {
    service
        .materialize_projection(
            &context_for("alice"),
            &ProjectionMaterializationRequest {
                projection: projection(),
                project_id: "proj-1".to_owned(),
                workspace_id: "ws-1".to_owned(),
            },
            adapter,
            coordinator,
        )
        .expect("materialization succeeds");
}

/// Reads the recorded synchronization state for the fixture projection.
///
/// - `coordinator`: Filesystem boundary holding the state.
///
/// # Returns
/// The synchronization state of the fixture projection.
///
/// # Panics
/// Panics when the state is absent.
fn recorded_state(coordinator: &SyncCoordinator<LocalFileSystem>) -> SyncState {
    coordinator
        .states()
        .find_projection(projection())
        .cloned()
        .expect("materialized projection state")
}

/// Verifies the product operation writes the workspace policy to disk, records the
/// applied style, and keeps a stored personal preference out of the file.
#[test]
fn materialization_writes_workspace_policy_bytes_to_disk() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let file = directory.path().join("document.txt");
    let (service, _store, _probe) = service();

    service
        .set_projection_style_user_preference(
            &context_for("alice"),
            &fixture::set_style(&[("width", "100"), ("emphasis", "italic")]),
        )
        .expect("personal preference stored");
    store_workspace_policy(&service, &[("width", "80")]);

    let adapter = StyleEchoAdapter::default();
    let mut coordinator = coordinator(&file, ProjectionOwnership::ManagedEditable);
    service
        .materialize_projection(
            &context_for("alice"),
            &ProjectionMaterializationRequest {
                projection: projection(),
                project_id: "proj-1".to_owned(),
                workspace_id: "ws-1".to_owned(),
            },
            &adapter,
            &mut coordinator,
        )
        .expect("materialization succeeds");

    let written = LocalFileSystem
        .read(&file)
        .expect("materialized file readable");
    let materialized_fields = fields(&StyleValue::new(written.clone()));
    assert_eq!(
        materialized_fields.get("width").map(String::as_str),
        Some("80"),
        "the workspace policy must reach the file: {materialized_fields:?}"
    );
    assert_eq!(
        materialized_fields.get("emphasis").map(String::as_str),
        Some("bold"),
        "the domain default must apply where the policy is silent: {materialized_fields:?}"
    );
    assert!(
        !String::from_utf8_lossy(&written).contains("italic")
            && !String::from_utf8_lossy(&written).contains("100"),
        "a stored personal preference must never be materialized: {}",
        String::from_utf8_lossy(&written)
    );

    let state = recorded_state(&coordinator);
    assert_eq!(adapter.calls.get(), 1, "exactly one render writes");
    assert_eq!(
        state.style,
        StyleRef::Value(StyleValue::new(written.clone()))
    );
    assert_eq!(state.rendered, digest_bytes(&written));
}

/// Verifies a second materialization under a different workspace policy rewrites
/// the same file with different bytes and records the new style.
#[test]
fn a_changed_workspace_policy_rewrites_the_materialized_bytes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let file = directory.path().join("document.txt");
    let (service, _store, _probe) = service();
    store_workspace_policy(&service, &[("width", "80")]);

    let adapter = StyleEchoAdapter::default();
    let mut coordinator = coordinator(&file, ProjectionOwnership::ManagedEditable);
    materialize(&service, &mut coordinator, &adapter);
    let first = LocalFileSystem
        .read(&file)
        .expect("first materialization readable");

    store_workspace_policy(&service, &[("indent", "tabs")]);
    materialize(&service, &mut coordinator, &adapter);
    let second = LocalFileSystem
        .read(&file)
        .expect("second materialization readable");

    assert_ne!(
        first, second,
        "one semantic target must materialize differently under another policy"
    );
    assert_eq!(
        fields(&StyleValue::new(second.clone()))
            .get("indent")
            .map(String::as_str),
        Some("tabs")
    );
    assert_eq!(
        recorded_state(&coordinator).style,
        StyleRef::Value(StyleValue::new(second)),
        "the recorded style must be the one the last render used"
    );
    assert_eq!(adapter.calls.get(), 2, "each materialization renders once");
}

/// Verifies the materialization receipt names only shared sources: a stored
/// personal preference for the same principal never enters the applied chain.
#[test]
fn the_materialization_receipt_carries_no_personal_source() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let file = directory.path().join("document.txt");
    let (service, _store, _probe) = service();
    service
        .set_projection_style_user_preference(
            &context_for("alice"),
            &fixture::set_style(&[("width", "100")]),
        )
        .expect("personal preference stored");
    store_workspace_policy(&service, &[("width", "80")]);

    let adapter = StyleEchoAdapter::default();
    let mut coordinator = coordinator(&file, ProjectionOwnership::ManagedEditable);
    let receipt = service
        .materialize_projection(
            &context_for("alice"),
            &ProjectionMaterializationRequest {
                projection: projection(),
                project_id: "proj-1".to_owned(),
                workspace_id: "ws-1".to_owned(),
            },
            &adapter,
            &mut coordinator,
        )
        .expect("materialization succeeds");

    assert_eq!(
        receipt
            .applied
            .iter()
            .map(|source| source.kind)
            .collect::<Vec<StyleSourceKind>>(),
        vec![
            StyleSourceKind::DomainDefault,
            StyleSourceKind::WorkspaceMaterializationPolicy
        ]
    );
    assert_eq!(receipt.location.path(), file.as_path());
    assert_eq!(
        receipt.resolved.value().as_bytes(),
        LocalFileSystem
            .read(&file)
            .expect("materialized file readable")
    );
}

/// Verifies the operation refuses to write a location the filesystem layer does
/// not treat as Scriberig-managed, leaving the existing bytes alone.
#[test]
fn materialization_refuses_an_unmanaged_location() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let file = directory.path().join("document.txt");
    LocalFileSystem
        .write_atomic(&file, b"not a scriberig projection")
        .expect("seed file written");
    let (service, _store, _probe) = service();
    store_workspace_policy(&service, &[("width", "80")]);

    let adapter = StyleEchoAdapter::default();
    let mut coordinator = coordinator(&file, ProjectionOwnership::Unmanaged);
    let error = service
        .materialize_projection(
            &context_for("alice"),
            &ProjectionMaterializationRequest {
                projection: projection(),
                project_id: "proj-1".to_owned(),
                workspace_id: "ws-1".to_owned(),
            },
            &adapter,
            &mut coordinator,
        )
        .expect_err("an unmanaged location must not be materialized");

    assert!(matches!(
        error,
        ApplicationError::InvalidRequest { operation, .. }
            if operation == "projection materialize"
    ));
    assert_eq!(
        LocalFileSystem
            .read(&file)
            .expect("seed file still readable"),
        b"not a scriberig projection"
    );
    assert_eq!(
        recorded_state(&coordinator).style,
        StyleRef::Value(StyleValue::new(Vec::new())),
        "a refused materialization must not record a style"
    );
}
