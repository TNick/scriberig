//! Verifies semantic DXF line edits use candidate validation and shared apply.

use std::fs;
use std::sync::Arc;

use super::dxf_project;
use scriberig_domain_cad::{CadEntity, Coordinate, DrawingEntity};
use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Routes line geometry changes through candidate validation and shared apply.
///
/// The coherent integration path includes invalid input, isolation, revision
/// identity, materialization, and stale-base assertions for both source lines.
#[allow(clippy::too_many_lines)]
#[test]
fn line_geometry_edit_is_candidate_only_until_validated_apply() {
    let root = dxf_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    service.import(project).expect("import DXF");
    let workspace = service.workspace(project).expect("active workspace");
    let source_path = root.join("roads.dxf");
    let original = fs::read(&source_path).expect("original source");
    let entity = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.entity")
        .expect("line entity");
    let revision_id = store
        .resolve(workspace.snapshot, entity.entity)
        .expect("resolve line")
        .expect("line revision");
    let revision = store
        .get(revision_id)
        .expect("load line revision")
        .expect("line payload");
    let line: DrawingEntity = serde_json::from_slice(revision.payload()).expect("decode line");
    let drawing_id = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.drawing")
        .expect("drawing entity")
        .entity;
    let initial_import =
        scriberig_domain_dxf::import_dxf(&original, drawing_id).expect("import source provenance");
    let native_handle = initial_import
        .provider_import
        .entity_identifier(entity.entity)
        .expect("external line handle")
        .value
        .clone();
    let invalid = service
        .propose_dxf_line_geometry(
            project,
            workspace.snapshot,
            entity.entity,
            &Coordinate::new("NaN", "3", "0"),
            &Coordinate::new("12", "13", "0"),
        )
        .expect_err("non-finite coordinate must be rejected");
    assert_eq!(invalid.code, "dxf_invalid_coordinate");
    assert_eq!(invalid.diagnostics.len(), 1);
    assert_eq!(invalid.diagnostics[0].entity, Some(entity.entity));
    assert_eq!(invalid.diagnostics[0].path, "roads.dxf");
    assert_eq!(invalid.diagnostics[0].stage, "edit");
    assert_eq!(
        fs::read(&source_path).expect("live source after invalid edit"),
        original
    );
    let unrelated = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .filter(|owned| owned.kind == "cad.entity" && owned.entity != entity.entity)
        .find_map(|owned| {
            let revision = store
                .resolve(workspace.snapshot, owned.entity)
                .expect("resolve unrelated entity")?;
            let payload = store.get(revision).expect("load unrelated revision")?;
            let semantic: DrawingEntity =
                serde_json::from_slice(payload.payload()).expect("decode unrelated entity");
            matches!(semantic.geometry, CadEntity::Line { .. }).then_some((owned.entity, revision))
        })
        .expect("unrelated line entity");
    let candidate = service
        .propose_dxf_line_geometry(
            project,
            workspace.snapshot,
            entity.entity,
            &Coordinate::new("2", "3", "0"),
            &Coordinate::new("12", "13", "0"),
        )
        .expect("propose line update");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("cad.replace_line_geometry")
    );
    let semantic_diff = &candidate.semantic_changes[0];
    assert_eq!(semantic_diff.target, entity.entity);
    assert_eq!(semantic_diff.property, "entity.geometry");
    assert_eq!(
        semantic_diff.before,
        serde_json::to_value(&line.geometry).expect("prior geometry JSON")
    );
    assert_eq!(
        semantic_diff.after,
        serde_json::to_value(CadEntity::Line {
            start: Coordinate::new("2", "3", "0"),
            end: Coordinate::new("12", "13", "0"),
        })
        .expect("proposed geometry JSON")
    );
    assert_eq!(
        service
            .candidate(&candidate.id)
            .expect("reload candidate")
            .expect("candidate")
            .semantic_changes,
        candidate.semantic_changes
    );
    assert_eq!(
        fs::read(&source_path).expect("live source before apply"),
        original
    );
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate DXF candidate");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("apply DXF candidate");
    assert_ne!(applied.successor_snapshot, workspace.snapshot);
    assert_eq!(
        store
            .change_provenance(applied.successor_snapshot)
            .expect("read CAD snapshot provenance")
            .expect("applied CAD change")
            .semantic_operation
            .as_deref(),
        Some("cad.replace_line_geometry")
    );
    assert_eq!(
        store
            .resolve(applied.successor_snapshot, unrelated.0)
            .expect("resolve unchanged line after apply"),
        Some(unrelated.1),
        "editing one line must not revise an unrelated entity"
    );
    let stale = service
        .propose_dxf_line_geometry(
            project,
            workspace.snapshot,
            entity.entity,
            &Coordinate::new("4", "5", "0"),
            &Coordinate::new("14", "15", "0"),
        )
        .expect_err("old base snapshot must not create a candidate");
    assert_eq!(stale.code, "stale_candidate");
    let reopened = scriberig_domain_dxf::import_dxf(
        &fs::read(&source_path).expect("materialized source"),
        drawing_id,
    )
    .expect("reopen materialized source");
    let edited_id = reopened
        .provider_import
        .entity_identifier(entity.entity)
        .expect("same external identifier")
        .entity;
    assert_eq!(
        reopened
            .provider_import
            .entity_identifier(entity.entity)
            .expect("same DXF native handle")
            .value,
        native_handle
    );
    let edited = reopened
        .drawing()
        .entities
        .iter()
        .find(|candidate| candidate.id == edited_id)
        .expect("same semantic entity");
    assert_eq!(
        edited.geometry,
        CadEntity::Line {
            start: Coordinate::new("2", "3", "0"),
            end: Coordinate::new("12", "13", "0"),
        }
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
