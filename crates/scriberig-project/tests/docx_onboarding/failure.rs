//! Verifies invalid staged DOCX packages never replace the live document.

use std::fs;
use std::sync::Arc;

use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, ProjectService, ProjectSourceChange,
    ProjectSourceChangeKind,
};
use scriberig_store_sqlite::SqliteStore;

use crate::docx_project;

/// Rejects staged DOCX corruption without replacing source or canonical history.
#[test]
fn corrupt_staged_docx_is_rejected_before_live_replacement() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let base = service.import(project).expect("import document").snapshot;
    let live_path = root.join("report.docx");
    let original = fs::read(&live_path).expect("original DOCX bytes");
    let candidate = service
        .propose_project_change(
            project,
            base,
            vec![ProjectSourceChange {
                path: "report.docx".into(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"corrupt staged DOCX package".to_vec()),
            }],
        )
        .expect("propose corrupt staged package");
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("record package validation failure");

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
