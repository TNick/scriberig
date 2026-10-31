//! Verifies staged `GeoPackage` provider failures cannot replace the live dataset.

use std::fs;
use std::sync::Arc;

use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, ProjectService, ProjectSourceChange,
    ProjectSourceChangeKind,
};
use scriberig_store_sqlite::SqliteStore;

use crate::geopackage_fixture;

/// Rejects an invalid staged `GeoPackage` without changing live bytes or history.
#[test]
fn invalid_staged_geopackage_is_rejected_before_live_replacement() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-geopackage-failure-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    fs::write(root.join("parcels.gpkg"), geopackage_fixture()).expect("write GeoPackage");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let base = service.import(project).expect("import GeoPackage").snapshot;
    let live_path = root.join("parcels.gpkg");
    let original = fs::read(&live_path).expect("original GeoPackage bytes");
    let candidate = service
        .propose_project_change(
            project,
            base,
            vec![ProjectSourceChange {
                path: "parcels.gpkg".into(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"provider cannot reopen this staged package".to_vec()),
            }],
        )
        .expect("propose invalid staged package");
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::GeoPackagePackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("record provider validation failure");

    assert!(matches!(
        validated.validation,
        CandidateValidation::Failed { .. }
    ));
    let persisted = service
        .candidate(&candidate.id)
        .expect("reload candidate validation")
        .expect("durable candidate");
    assert!(matches!(
        persisted.validation,
        CandidateValidation::Failed { ref diagnostics } if !diagnostics.is_empty()
    ));
    assert_eq!(fs::read(&live_path).expect("live source remains"), original);
    assert_eq!(
        store
            .project(project)
            .expect("project state")
            .expect("project")
            .current_snapshot,
        Some(base)
    );
    assert!(service.apply_candidate(&validated).is_err());
    assert!(
        service.apply_candidate(&validated).is_err(),
        "retrying a failed package validation must never report success"
    );
    assert_eq!(
        fs::read(&live_path).expect("source after rejected apply"),
        original
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
