//! Proves durable style settings resolve for a later service on the same database.

#[allow(dead_code)] // This binary uses a subset of the shared style fixtures.
#[path = "projection_style_fixture.rs"]
mod fixture;

use std::sync::Arc;

use scriberig_app::{
    ApplicationService, ProjectionStyleBackend, ProjectionStyleConfiguration,
    ProjectionStyleSettingRequest,
};
use scriberig_style_sqlite::SqliteProjectionStyleStore;
use sr_projection::ProjectionPurpose;

use fixture::{DocumentStyleAdapter, RecordingBackend, context_for, effective_fields, kind, style};

/// Builds one service whose style settings live in the durable store at `path`.
///
/// - `path`: SQLite database path holding the style settings.
///
/// # Returns
/// The service and a backend probe counting canonical requests.
fn service_on(path: &std::path::Path) -> (ApplicationService<RecordingBackend>, RecordingBackend) {
    let store =
        Arc::new(SqliteProjectionStyleStore::open(path).expect("durable style store opens"));
    let backend = RecordingBackend::default();
    let probe = backend.clone();
    let service = ApplicationService::new(backend).with_projection_style_configuration(
        ProjectionStyleConfiguration::new(Arc::new(DocumentStyleAdapter), store),
    );
    (service, probe)
}

/// Verifies a stored preference resolves in a service built after the write.
///
/// A second store is opened on the same file, which is what a restarted process
/// does; an in-memory store could not satisfy this.
#[test]
fn a_preference_written_by_one_service_resolves_in_a_later_service() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("scriberig.db");

    let (writer, probe) = service_on(&path);
    writer
        .set_projection_style_user_preference(
            &context_for("alice"),
            &fixture::set_style(&[("width", "100")]),
        )
        .expect("preference written");
    assert_eq!(
        probe.seen(),
        0,
        "a style setting write must open no canonical operation"
    );

    let (reader, _probe) = service_on(&path);
    let resolved = effective_fields(
        &reader,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        None,
    );
    assert_eq!(
        resolved.get("width").map(String::as_str),
        Some("100"),
        "the durable preference must resolve after a reopen"
    );
}

/// Verifies a durable workspace policy applies to shared purposes only.
#[test]
fn a_durable_materialization_policy_applies_to_shared_purposes_only() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("scriberig.db");

    let (writer, _probe) = service_on(&path);
    let policy =
        ProjectionStyleSettingRequest::new(kind(), "ws-1", Some(style(&[("width", "100")])));
    writer
        .set_projection_style_materialization_policy(&context_for("alice"), &policy)
        .expect("materialization policy written");

    let (reader, _probe) = service_on(&path);
    let shared = effective_fields(
        &reader,
        &context_for("alice"),
        ProjectionPurpose::SharedMaterialization,
        None,
    );
    let view = effective_fields(
        &reader,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        None,
    );
    assert_eq!(shared.get("width").map(String::as_str), Some("100"));
    assert_eq!(
        view.get("width").map(String::as_str),
        Some("80"),
        "a workspace materialization policy must not restyle a personal view"
    );
}

/// Verifies one principal's durable preference never resolves for another.
#[test]
fn a_durable_preference_stays_inside_its_own_principal_scope() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("scriberig.db");

    let (writer, _probe) = service_on(&path);
    writer
        .set_projection_style_user_preference(
            &context_for("alice"),
            &fixture::set_style(&[("width", "100")]),
        )
        .expect("preference written");

    let (reader, _probe) = service_on(&path);
    let other = effective_fields(
        &reader,
        &context_for("bob"),
        ProjectionPurpose::InteractiveView,
        None,
    );
    assert_eq!(other.get("width").map(String::as_str), Some("80"));
}
