//! Exercises the checked-in DXF through shared candidate validation and materialization.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use scriberig_domain_cad::{CadEntity, Coordinate, DrawingEntity};
use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

#[allow(
    clippy::too_many_lines,
    reason = "This integration test exercises a full real-fixture edit, persistence, reopen, and fidelity verification lifecycle."
)]
/// Edits a fresh fixture copy, applies it, and verifies reopened semantic fidelity.
#[test]
fn checked_in_dxf_golden_edit_reopens_without_mutating_fixture() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/drawing.dxf");
    let fixture = fixture.canonicalize().expect("resolve checked-in fixture");
    let original_fixture = fs::read(&fixture).expect("read checked-in fixture");
    let root = std::env::temp_dir().join(format!(
        "scriberig-dxf-golden-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create isolated project");
    let source_path = root.join("drawing.dxf");
    fs::copy(&fixture, &source_path).expect("copy fixture into isolated project");
    let original = fs::read(&source_path).expect("read isolated source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan copied fixture");
    service
        .import(scan.project.id)
        .expect("import copied fixture");
    let workspace = service
        .workspace(scan.project.id)
        .expect("project workspace");
    let ownership = store
        .project_entities(scan.project.id)
        .expect("project semantic entities");
    let drawing_id = ownership
        .iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("CAD drawing")
        .entity;
    let original_drawing = service
        .cad_drawing(scan.project.id, workspace.snapshot, drawing_id)
        .expect("inspect original drawing");
    assert!(
        original_drawing
            .layers
            .iter()
            .any(|layer| layer.name == "PARCELS")
    );
    assert!(
        original_drawing
            .layers
            .iter()
            .any(|layer| layer.name == "LABELS")
    );
    let line = ownership
        .iter()
        .filter(|entity| entity.kind == "cad.entity")
        .find_map(|owned| {
            let revision = store.resolve(workspace.snapshot, owned.entity).ok()??;
            let payload = store.get(revision).ok()??;
            let entity: DrawingEntity = serde_json::from_slice(payload.payload()).ok()?;
            matches!(entity.geometry, CadEntity::Line { .. }).then_some((owned.entity, entity))
        })
        .expect("fixture line entity");
    let unrelated = service
        .cad_entities(scan.project.id, workspace.snapshot, drawing_id, 0, 100)
        .expect("list original drawing entities")
        .entities
        .into_iter()
        .filter(|entity| entity.id != line.0)
        .collect::<Vec<_>>();
    assert!(
        unrelated
            .iter()
            .any(|entity| matches!(entity.geometry, CadEntity::Polyline { .. }))
    );
    assert!(
        unrelated
            .iter()
            .any(|entity| matches!(entity.geometry, CadEntity::Text { .. }))
    );

    let candidate = service
        .propose_dxf_line_geometry(
            scan.project.id,
            workspace.snapshot,
            line.0,
            &Coordinate::new("1.25", "2.5", "0"),
            &Coordinate::new("5.25", "2.5", "0"),
        )
        .expect("create semantic line candidate");
    assert_eq!(
        fs::read(&source_path).expect("live source before apply"),
        original
    );
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DxfPackage,
            &AtomicBool::new(false),
        )
        .expect("validate staged DXF");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    service
        .apply_candidate(&validated)
        .expect("apply candidate");

    let reopened_bytes = fs::read(&source_path).expect("materialized source");
    let reopened = scriberig_domain_dxf::import_dxf(&reopened_bytes, drawing_id)
        .expect("reopen materialized DXF");
    let target = reopened
        .drawing()
        .entities
        .iter()
        .find(|entity| entity.id == line.0)
        .expect("same target semantic identity after reopen");
    assert_eq!(
        target.geometry,
        CadEntity::Line {
            start: Coordinate::new("1.25", "2.5", "0"),
            end: Coordinate::new("5.25", "2.5", "0"),
        }
    );
    for expected in &unrelated {
        assert!(
            reopened.drawing().entities.contains(expected),
            "unrelated supported entity {:?} must survive materialization",
            expected.id
        );
    }
    assert_eq!(
        fs::read(&fixture).expect("checked-in fixture after test"),
        original_fixture
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove isolated project");
}
