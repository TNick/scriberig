//! Proves materialized files carry the exact style the caller supplies.

use std::cell::Cell;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    ProjectionAdapter, ProjectionCapabilities, ProjectionKind, ProjectionRef, ProjectionResult,
    ProjectionStatus, RenderRequest, RenderedProjection, ResolvedStyle, SemanticStateRef,
    StyleValue,
};
use sr_projection_fs::{
    FileSystem, LocalFileSystem, MemorySyncStore, ProjectionLocation, ProjectionOwnership,
    SyncCoordinator, SyncState,
};

mod common;

/// Counter naming isolated temporary roots for these tests.
static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

/// Builds one projection identity used by these tests.
///
/// # Returns
/// The projection reference.
fn projection() -> ProjectionRef {
    ProjectionRef(EntityId::from_bytes([1; 16]))
}

/// Adapter that echoes the exact style bytes into the rendered file, making a
/// materialization style observable in the written bytes.
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

    /// Renders the exact style bytes into the file body.
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
        let width = std::str::from_utf8(request.style.value().as_bytes()).unwrap_or("?");
        Ok(RenderedProjection::from_bytes(
            format!("width={width}").into_bytes(),
            request,
            None,
        ))
    }
}

/// Builds one coordinator owning one managed projection stored at `path`.
///
/// - `path`: Managed file location.
///
/// # Returns
/// The coordinator with its synchronization state.
fn coordinator(path: &std::path::Path) -> SyncCoordinator<LocalFileSystem> {
    let mut states = MemorySyncStore::default();
    states.put(SyncState::new(
        projection(),
        common::baseline("rust-source", SnapshotId::new([2; 32]), b"80", b"seed"),
        ProjectionLocation::new(path),
        ProjectionOwnership::ManagedEditable,
        SemanticStateRef(SnapshotId::new([3; 32])),
        ProjectionStatus::Synchronized,
    ));
    SyncCoordinator::new(LocalFileSystem, states)
}

/// Builds one fresh temporary file location for one test.
///
/// - `label`: Test label naming the temporary root.
///
/// # Returns
/// The path, whose parent directory is created.
fn temporary_path(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-fs-style-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("temporary root");
    root.join("projection.rs")
}

/// Verifies two workspace materialization styles write different bytes for one
/// semantic target, and that the recorded style tracks the last render.
#[test]
fn each_materialization_style_writes_its_own_bytes() {
    let path = temporary_path("two-styles");
    let mut coordinator = coordinator(&path);
    let adapter = StyleEchoAdapter::default();

    coordinator
        .render_managed_projection_with_style(
            projection(),
            &adapter,
            &ResolvedStyle::inline(StyleValue::new(b"80".to_vec())),
        )
        .expect("first materialization");
    assert_eq!(
        LocalFileSystem.read(&path).expect("read"),
        b"width=80",
        "the supplied style must reach the materialized file"
    );

    coordinator
        .render_managed_projection_with_style(
            projection(),
            &adapter,
            &ResolvedStyle::inline(StyleValue::new(b"100".to_vec())),
        )
        .expect("second materialization");
    let written = LocalFileSystem.read(&path).expect("read");
    assert_eq!(
        written, b"width=100",
        "a changed workspace policy must not be served from the previous render"
    );

    let state = coordinator
        .states()
        .find_projection(projection())
        .expect("state");
    assert_eq!(
        state.baseline.rendered.style.value().as_bytes(),
        b"100",
        "reconciliation identity must track the style that produced the bytes"
    );
    assert_eq!(state.baseline.rendered.bytes, written);
    assert_eq!(adapter.calls.get(), 2);
    let _ = fs::remove_dir_all(path.parent().expect("parent"));
}

/// Verifies the style-free entry point uses the recorded style and never invents
/// one of its own, so personal state can only arrive through the caller.
#[test]
fn the_state_entry_point_renders_the_recorded_style_only() {
    let path = temporary_path("recorded-style");
    let mut coordinator = coordinator(&path);
    let adapter = StyleEchoAdapter::default();

    coordinator
        .render_managed_projection(projection(), &adapter)
        .expect("render from recorded state");
    assert_eq!(LocalFileSystem.read(&path).expect("read"), b"width=80");

    coordinator
        .render_managed_projection_with_style(
            projection(),
            &adapter,
            &ResolvedStyle::inline(StyleValue::new(b"100".to_vec())),
        )
        .expect("render with explicit style");
    coordinator
        .render_managed_projection(projection(), &adapter)
        .expect("render again from recorded state");
    assert_eq!(
        LocalFileSystem.read(&path).expect("read"),
        b"width=100",
        "a later state-driven render reuses the shared style that was recorded"
    );
    let _ = fs::remove_dir_all(path.parent().expect("parent"));
}
