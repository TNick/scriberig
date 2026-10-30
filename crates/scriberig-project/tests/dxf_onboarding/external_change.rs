//! Verifies DXF candidates reject external edits made after validation.

use std::fs;
use std::sync::Arc;

use scriberig_domain_cad::Coordinate;
use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store_sqlite::SqliteStore;

use super::dxf_project;

#[allow(
    clippy::too_many_lines,
    reason = "This integration test verifies validation, concurrent external modification, and fail-closed application together."
)]
/// Preserves a concurrent external DXF edit when a validated candidate is applied.
#[test]
fn external_dxf_change_after_validation_blocks_replacement() {
    let root = dxf_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    service.import(project).expect("import DXF");
    let workspace = service.workspace(project).expect("active workspace");
    let source_path = root.join("roads.dxf");
    let original = fs::read(&source_path).expect("original DXF bytes");
    let target = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "cad.entity")
        .expect("line entity");
    let candidate = service
        .propose_dxf_line_geometry(
            project,
            workspace.snapshot,
            target.entity,
            &Coordinate::new("2", "3", "0"),
            &Coordinate::new("12", "13", "0"),
        )
        .expect("propose line candidate");
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate candidate");
    assert_eq!(validated.validation, CandidateValidation::Passed);

    let external_bytes = [original.as_slice(), b"\n999\nexternal-owner-edit\n"].concat();
    fs::write(&source_path, &external_bytes).expect("write concurrent external edit");
    let error = service
        .apply_candidate(&validated)
        .expect_err("external source change must block replacement");
    assert_eq!(error.code, "filesystem_conflict");
    assert!(error.message.contains("roads.dxf"));
    let conflict = service
        .materialization_receipt(&candidate.id)
        .expect("load conflict receipt")
        .expect("durable conflict evidence")
        .paths
        .into_iter()
        .find(|path| path.path == "roads.dxf")
        .expect("conflicting DXF path");
    assert_eq!(conflict.state, "conflict");
    let durable_path = store
        .project_effect_paths(&candidate.id)
        .expect("load exact path evidence")
        .into_iter()
        .find(|record| record.path == "roads.dxf")
        .expect("persisted DXF evidence");
    assert_eq!(
        durable_path.expected_digest,
        Some(*blake3::hash(&original).as_bytes()),
        "conflict evidence identifies the candidate baseline"
    );
    assert_eq!(
        durable_path.observed_digest,
        Some(*blake3::hash(&external_bytes).as_bytes()),
        "conflict evidence identifies the competing external artifact"
    );
    let persisted_candidate = service
        .candidate(&candidate.id)
        .expect("load candidate evidence")
        .expect("durable candidate");
    assert_eq!(persisted_candidate.base_snapshot, workspace.snapshot);
    assert_eq!(persisted_candidate.semantic_target, Some(target.entity));
    assert_eq!(
        persisted_candidate.semantic_operation.as_deref(),
        Some("cad.replace_line_geometry")
    );
    assert_eq!(
        fs::read(&source_path).expect("external source remains intact"),
        external_bytes
    );
    assert_eq!(
        service
            .workspace(project)
            .expect("workspace after conflict")
            .snapshot,
        workspace.snapshot,
        "rejected replacement must not advance canonical state"
    );

    fs::write(&source_path, &original).expect("restore exact validated baseline");
    let recovered = service
        .apply_candidate(&validated)
        .expect("retry after restoring the exact baseline");
    assert_ne!(recovered.successor_snapshot, workspace.snapshot);
    assert_eq!(
        service
            .workspace(project)
            .expect("workspace after retry")
            .snapshot,
        recovered.successor_snapshot,
        "successful retry advances canonical state exactly once"
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
