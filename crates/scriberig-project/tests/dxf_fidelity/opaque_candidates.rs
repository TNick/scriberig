//! Verifies candidates preserve omitted DXF source content.

use super::*;

/// Edits supported entities while exact candidate validation retains unknown records.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "This integration test verifies opaque preservation across both line and layer candidate lifecycles."
)]
/// Preserves unsupported bytes across validated semantic edits.
fn unsupported_content_survives_validated_dxf_candidates() {
    let root = dxf_project_with_unsupported_entity();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    service.import(project).expect("import DXF");
    let manifest = service
        .artifacts(project)
        .expect("artifact fidelity")
        .into_iter()
        .find(|artifact| artifact.source.adapter == "dxf")
        .expect("DXF artifact");
    assert_eq!(
        manifest.fidelity.capability,
        scriberig_project::ArtifactCapability::EditableWithOpaquePreservation
    );
    assert_eq!(
        manifest.fidelity.opaque_preservation,
        scriberig_project::OpaquePreservation::Preserved
    );
    assert!(
        manifest
            .fidelity
            .diagnostic
            .as_deref()
            .is_some_and(|diagnostic| diagnostic.contains("unsupported DXF entity"))
    );
    let workspace = service.workspace(project).expect("workspace");
    let entity = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.entity")
        .expect("supported line");
    let source_path = root.join("roads.dxf");
    let original = fs::read(&source_path).expect("original DXF bytes");
    let drawing_id = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.drawing")
        .expect("drawing")
        .entity;
    let imported = scriberig_domain_dxf::import_dxf(&original, drawing_id).expect("import base");
    let handle = imported
        .provider_import
        .entity_identifier(entity.entity)
        .expect("selected native handle")
        .value
        .clone();
    let proposed = service
        .propose_dxf_line_geometry(
            project,
            workspace.snapshot,
            entity.entity,
            &Coordinate::new("2", "3", "0"),
            &Coordinate::new("12", "13", "0"),
        )
        .expect("opaque-preserving line candidate");
    let validated = service
        .validate_candidate_profile_cancellable(
            &proposed.id,
            scriberig_project::CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate opaque-preserving line candidate");
    assert_eq!(
        validated.validation,
        scriberig_project::CandidateValidation::Passed
    );
    scriberig_domain_dxf::verify_opaque_preservation(
        &original,
        &validated.new,
        &handle,
        &[10, 20, 30, 11, 21, 31],
    )
    .expect("candidate preserves every unselected source byte");
    let applied = service
        .apply_candidate(&validated)
        .expect("apply opaque-preserving line candidate");
    let after_apply = fs::read(&source_path).expect("materialized output");
    assert_eq!(after_apply, validated.new);
    let next_snapshot = applied.successor_snapshot;
    let layer = store
        .project_entities(project)
        .expect("semantic layers")
        .into_iter()
        .filter(|owned| owned.kind == "cad.layer")
        .find_map(|owned| {
            let revision = store
                .resolve(next_snapshot, owned.entity)
                .expect("resolve layer")?;
            let record = store.get(revision).expect("load layer")?;
            let layer: scriberig_domain_cad::Layer =
                serde_json::from_slice(record.payload()).expect("decode layer");
            (layer.name == "LABELS").then_some(owned)
        })
        .expect("destination layer");
    let layer_candidate = service
        .propose_dxf_layer_change(project, next_snapshot, entity.entity, layer.entity)
        .expect("opaque-preserving layer candidate");
    let validated_layer = service
        .validate_candidate_profile_cancellable(
            &layer_candidate.id,
            scriberig_project::CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate opaque-preserving layer candidate");
    assert_eq!(
        validated_layer.validation,
        scriberig_project::CandidateValidation::Passed
    );
    scriberig_domain_dxf::verify_opaque_preservation(
        &after_apply,
        &validated_layer.new,
        &handle,
        &[8],
    )
    .expect("layer candidate preserves every unselected source byte");
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Returns a structured candidate failure when an opaque record is altered.
#[test]
fn opaque_candidate_mutation_fails_with_preservation_diagnostic() {
    let root = super::dxf_project_with_unsupported_entity();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan").project.id;
    let imported = service.import(project).expect("import");
    let entity = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.entity")
        .expect("supported target");
    let candidate = service
        .propose_dxf_line_geometry(
            project,
            imported.snapshot,
            entity.entity,
            &Coordinate::new("2", "3", "0"),
            &Coordinate::new("12", "13", "0"),
        )
        .expect("propose line edit");
    let mut durable = store
        .project_candidate(&candidate.id)
        .expect("load durable candidate")
        .expect("candidate record");
    mutate_opaque_block(&mut durable.new);
    mutate_opaque_block(
        durable.effects[0]
            .new
            .as_mut()
            .expect("candidate effect output"),
    );
    store
        .put_project_candidate(&durable)
        .expect("persist altered candidate");
    let failed = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            scriberig_project::CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validation reports candidate failure");
    let scriberig_project::CandidateValidation::Failed { diagnostics } = failed.validation else {
        panic!("opaque mutation must fail candidate validation");
    };
    assert!(
        diagnostics
            .join(" ")
            .contains("dxf_opaque_preservation_failed")
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Rejects an opaque-preserving candidate that disagrees with its recorded edit.
#[test]
fn staged_line_values_must_match_the_recorded_semantic_change() {
    let root = super::dxf_project_with_unsupported_entity();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan").project.id;
    let imported = service.import(project).expect("import");
    let entity = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.entity")
        .expect("supported line");
    let candidate = service
        .propose_dxf_line_geometry(
            project,
            imported.snapshot,
            entity.entity,
            &Coordinate::new("2", "3", "0"),
            &Coordinate::new("12", "13", "0"),
        )
        .expect("propose line edit");
    let drawing_id = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.drawing" && owned.relative_path == candidate.path)
        .expect("drawing identity")
        .entity;
    let base = scriberig_domain_dxf::import_dxf(&candidate.old, drawing_id).expect("import base");
    let handle = base
        .provider_import
        .entity_identifier(entity.entity)
        .expect("native handle")
        .value
        .clone();
    let altered = scriberig_domain_dxf::replace_line_geometry(
        &candidate.old,
        &handle,
        &Coordinate::new("100", "101", "0"),
        &Coordinate::new("110", "111", "0"),
    )
    .expect("construct valid but intent-mismatched source");
    let mut durable = store
        .project_candidate(&candidate.id)
        .expect("load durable candidate")
        .expect("candidate record");
    durable.new.clone_from(&altered);
    durable.effects[0]
        .new
        .as_mut()
        .expect("candidate effect output")
        .clone_from(&altered);
    store
        .put_project_candidate(&durable)
        .expect("persist altered candidate");
    let failed = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            scriberig_project::CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validation records a failed candidate");
    let scriberig_project::CandidateValidation::Failed { diagnostics } = failed.validation else {
        panic!("candidate values must match recorded semantic intent");
    };
    assert!(diagnostics.join(" ").contains("candidate semantic diff"));
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Changes an opaque block name without changing the DXF pair structure.
///
/// # Parameters
/// - `bytes`: Staged candidate bytes to corrupt for the negative fixture.
fn mutate_opaque_block(bytes: &mut [u8]) {
    let marker = b"UNMODELED";
    let offset = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("opaque block record remains present");
    bytes[offset] = b'X';
}
