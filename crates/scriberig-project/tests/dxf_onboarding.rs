//! Verifies DXF scans import into separate drawing, layer, and entity records.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

#[path = "dxf_onboarding/external_change.rs"]
mod external_change;
#[path = "dxf_onboarding/failure.rs"]
mod failure;
#[path = "dxf_onboarding/golden_fixture.rs"]
mod golden_fixture;
#[path = "dxf_onboarding/line_geometry.rs"]
mod line_geometry;
#[path = "dxf_onboarding/text_update.rs"]
mod text_update;

/// Creates a unique project containing two deterministic DXF lines.
///
/// # Returns
/// Temporary project directory containing the generated drawing.
fn dxf_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-dxf-onboarding-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut drawing = dxf::Drawing::new();
    drawing.add_layer(dxf::tables::Layer {
        name: "PARCELS".to_owned(),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "LABELS".to_owned(),
        ..Default::default()
    });
    let mut first =
        dxf::entities::Entity::new(dxf::entities::EntityType::Line(dxf::entities::Line {
            p1: dxf::Point::new(0.0, 0.0, 0.0),
            p2: dxf::Point::new(10.0, 0.0, 0.0),
            ..Default::default()
        }));
    "PARCELS".clone_into(&mut first.common.layer);
    drawing.add_entity(first);
    let mut second =
        dxf::entities::Entity::new(dxf::entities::EntityType::Line(dxf::entities::Line {
            p1: dxf::Point::new(20.0, 0.0, 0.0),
            p2: dxf::Point::new(30.0, 0.0, 0.0),
            ..Default::default()
        }));
    "PARCELS".clone_into(&mut second.common.layer);
    drawing.add_entity(second);
    let mut bytes = Vec::new();
    drawing.save(&mut bytes).expect("serialize DXF");
    fs::write(root.join("roads.dxf"), bytes).expect("persist DXF");
    root
}

/// Changes one entity's layer through the normal project candidate lifecycle.
#[test]
fn layer_change_is_candidate_only_until_validated_apply() {
    let root = dxf_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    service.import(project).expect("import DXF");
    let workspace = service.workspace(project).expect("active workspace");
    let source_path = root.join("roads.dxf");
    let original = fs::read(&source_path).expect("original source");
    let owned = store.project_entities(project).expect("semantic entities");
    let target = owned
        .iter()
        .find(|item| item.kind == "cad.entity")
        .expect("target entity");
    let destination = owned
        .iter()
        .filter(|item| item.kind == "cad.layer")
        .find_map(|item| {
            let revision = store
                .resolve(workspace.snapshot, item.entity)
                .expect("resolve layer")?;
            let record = store.get(revision).expect("load layer")?;
            let layer: scriberig_domain_cad::Layer =
                serde_json::from_slice(record.payload()).expect("decode layer");
            (layer.name == "LABELS").then_some(item.entity)
        })
        .expect("destination layer");
    let entity_revision = store
        .resolve(workspace.snapshot, target.entity)
        .expect("resolve target")
        .expect("target revision");
    let drawing_id = owned
        .iter()
        .find(|item| item.kind == "cad.drawing" && item.relative_path == target.relative_path)
        .expect("drawing for target")
        .entity;
    let source_import =
        scriberig_domain_dxf::import_dxf(&original, drawing_id).expect("source provider import");
    let native_handle = source_import
        .provider_import
        .entity_identifier(target.entity)
        .expect("target external handle")
        .value
        .clone();
    let unrelated = owned
        .iter()
        .find(|item| item.kind == "cad.entity" && item.entity != target.entity)
        .map(|item| {
            (
                item.entity,
                store
                    .resolve(workspace.snapshot, item.entity)
                    .expect("resolve unrelated entity")
                    .expect("unrelated entity revision"),
            )
        })
        .expect("neighboring entity");

    let candidate = service
        .propose_dxf_layer_change(project, workspace.snapshot, target.entity, destination)
        .expect("propose layer change");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("cad.change_layer")
    );
    assert_eq!(
        fs::read(&source_path).expect("live source remains unchanged"),
        original
    );
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate layer change");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("apply layer change");
    assert_materialized_layer_change(
        &store,
        applied.successor_snapshot,
        target.entity,
        entity_revision,
        unrelated,
        &native_handle,
        &source_path,
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Verifies candidate provenance, semantic identity, and materialized DXF bytes.
///
/// # Parameters
/// - `service`: Project service after candidate application.
/// - `store`: Durable semantic snapshot store.
/// - `project`: Project receiving the update.
/// - `successor`: Exact successor snapshot returned by Apply.
/// - `target`: Semantic entity whose layer changed.
/// - `entity_revision`: Exact base revision of the target entity.
/// - `unrelated`: Unchanged neighboring entity and base revision.
/// - `native_handle`: Exact CAD handle preserved by the source writer.
/// - `source_path`: Materialized DXF path.
fn assert_materialized_layer_change(
    store: &SqliteStore,
    successor: SnapshotId,
    target: EntityId,
    entity_revision: RevisionId,
    unrelated: (EntityId, RevisionId),
    native_handle: &str,
    source_path: &Path,
) {
    assert_eq!(
        store
            .change_provenance(successor)
            .expect("read provenance")
            .expect("candidate provenance")
            .semantic_operation
            .as_deref(),
        Some("cad.change_layer")
    );
    assert_eq!(
        store
            .resolve(successor, unrelated.0)
            .expect("resolve unchanged neighbor"),
        Some(unrelated.1),
        "moving one entity must not revise an unrelated entity"
    );
    assert_ne!(
        store
            .resolve(successor, target)
            .expect("resolve moved entity"),
        Some(entity_revision),
        "the moved entity receives a new semantic revision"
    );
    let reopened = scriberig_domain_dxf::import_dxf(
        &fs::read(source_path).expect("materialized source"),
        scriberig_core::EntityId::new(),
    )
    .expect("reopen materialized DXF");
    let entity_id = reopened
        .provider_import
        .entity_identifiers()
        .iter()
        .find(|identifier| identifier.value == native_handle)
        .expect("external handle preserved")
        .entity;
    assert!(reopened.drawing().entities.iter().any(|entity| {
        entity.id == entity_id
            && reopened
                .drawing()
                .layers
                .iter()
                .any(|layer| layer.id == entity.layer && layer.name == "LABELS")
    }));
}
