//! Verifies QGIS layer ordering uses typed project candidates and shared apply.

use std::fs;
use std::sync::Arc;

use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store_sqlite::SqliteStore;

use super::support::mixed_project;

/// Applies a reordered flat QGIS layer tree through candidate review.
#[test]
fn qgis_layer_order_is_a_typed_candidate_and_shared_apply() {
    let root = mixed_project();
    let source = br#"<qgis><title>Planning</title><projectlayers><maplayer><id>parcel-layer</id><layername>Parcels</layername><datasource>./parcels.gpkg|layername=parcels</datasource><provider>ogr</provider></maplayer><maplayer><id>road-layer</id><layername>Roads</layername><datasource>./roads.shp</datasource><provider>ogr</provider></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-layer id="parcel-layer" name="Parcels"/><layer-tree-layer id="road-layer" name="Roads"/></layer-tree-group></qgis>"#;
    fs::write(root.join("project.qgs"), source).expect("write flat QGIS project");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let imported = service.import(project).expect("import project");
    let qgis = service
        .qgis_project(project, "project.qgs", imported.snapshot)
        .expect("read exact QGIS project");
    let before_order = qgis
        .project
        .layers
        .iter()
        .map(|layer| layer.id)
        .collect::<Vec<_>>();
    let requested_order = before_order.iter().rev().copied().collect::<Vec<_>>();
    let original_bytes = fs::read(root.join("project.qgs")).expect("read source");

    let candidate = service
        .propose_qgis_layer_order(project, imported.snapshot, "project.qgs", &requested_order)
        .expect("propose layer order");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("qgis.reorder_project_layers")
    );
    super::semantic_candidate_assertions::assert_persisted_semantic_change(
        &service,
        &candidate,
        qgis.project.id,
        "project.layer_order",
        &serde_json::json!(before_order),
        &serde_json::json!(requested_order),
    );
    assert_eq!(candidate.validation, CandidateValidation::Pending);
    assert_eq!(
        fs::read(root.join("project.qgs")).expect("source before apply"),
        original_bytes
    );

    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::QgisPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate layer order");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("apply candidate");
    let reopened = service
        .qgis_project(project, "project.qgs", applied.successor_snapshot)
        .expect("read successor project");
    assert_eq!(
        reopened
            .project
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>(),
        requested_order
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove temporary database");
    fs::remove_dir_all(root).expect("remove temporary project");
}
