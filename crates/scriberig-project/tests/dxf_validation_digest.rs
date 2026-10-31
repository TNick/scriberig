//! Verifies DXF validation binds exact candidate bytes before apply.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use scriberig_domain_cad::{CadEntity, Coordinate, DrawingEntity};
use scriberig_project::{CandidateValidationProfile, ProjectService};
use scriberig_store_sqlite::SqliteStore;

/// Copies the rich-project DXF fixture into a fresh temporary project.
///
/// # Returns
/// Temporary project directory containing one exact DXF source.
fn dxf_project() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-dxf-validation-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    fs::write(
        root.join("drawing.dxf"),
        include_bytes!("../../../tests/fixtures/rich-project/drawing.dxf"),
    )
    .expect("copy DXF fixture bytes");
    root
}

/// Rejects apply when durable DXF output changes after successful validation.
#[test]
fn changed_dxf_output_fails_exact_validation_digest_check() {
    let root = dxf_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let imported = service.import(project).expect("import DXF");
    let drawing = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("semantic CAD drawing");
    let page = service
        .cad_entities(project, imported.snapshot, drawing.entity, 0, 100)
        .expect("bounded CAD entities");
    let target = page.entities.first().expect("semantic CAD entity");
    let semantic: DrawingEntity = target.clone();
    let CadEntity::Line { start, end } = semantic.geometry else {
        panic!("fixture target should be a line");
    };
    let candidate = service
        .propose_dxf_line_geometry(
            project,
            imported.snapshot,
            target.id,
            &Coordinate::new("1", "2", start.z),
            &Coordinate::new("11", "2", end.z),
        )
        .expect("propose semantic line update");
    let live_before = fs::read(root.join("drawing.dxf")).expect("live DXF bytes");
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DxfPackage,
            &AtomicBool::new(false),
        )
        .expect("validate exact staged output");
    assert!(validated.validation_digest.is_some());

    let mut durable = store
        .project_candidate(&candidate.id)
        .expect("read durable candidate")
        .expect("candidate exists");
    durable.new.push(0);
    durable.effects[0]
        .new
        .as_mut()
        .expect("candidate effect output")
        .push(0);
    store
        .put_project_candidate(&durable)
        .expect("persist simulated output corruption");
    let changed = service
        .candidate(&candidate.id)
        .expect("reload candidate")
        .expect("candidate remains available");
    let error = service
        .apply_candidate(&changed)
        .expect_err("changed bytes require revalidation");

    assert_eq!(error.code, "validation_digest_mismatch");
    assert_eq!(
        fs::read(root.join("drawing.dxf")).expect("live source unchanged"),
        live_before
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
