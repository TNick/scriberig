//! Verifies CAD and geospatial candidates in the canonical mixed project.

use std::{fs, sync::Arc};

use scriberig_domain_geo::{GeoPackageReader, GeoValue};
use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, GeoPackageFeatureAttributeEdit, ProjectService,
};
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

use super::support::mixed_project;

/// Applies CAD and `GeoPackage` edits through shared candidate validation in one project.
#[allow(clippy::too_many_lines)]
#[test]
fn mixed_project_applies_cad_and_geo_candidates_with_isolation() {
    let root = mixed_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service
        .scan(None, &root)
        .expect("scan mixed project")
        .project
        .id;
    service.import(project).expect("import mixed project");
    let initial_workspace = service.workspace(project).expect("initial workspace");
    let initial = initial_workspace.snapshot;
    let entities = store.project_entities(project).expect("project entities");

    let drawing_record = entities
        .iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("CAD drawing identity");
    let drawing = service
        .cad_drawing(project, initial, drawing_record.entity)
        .expect("exact-snapshot drawing view");
    let parcels_layer = drawing
        .layers
        .iter()
        .find(|layer| layer.name == "PARCELS")
        .expect("PARCELS layer");
    let labels_layer = drawing
        .layers
        .iter()
        .find(|layer| layer.name == "LABELS")
        .expect("LABELS layer");
    let page = service
        .cad_entities(project, initial, drawing.drawing, 0, drawing.entity_count)
        .expect("bounded drawing entity page");
    let target = page
        .entities
        .iter()
        .find(|entity| entity.layer == parcels_layer.id)
        .expect("entity on PARCELS layer");
    let target_revision = store
        .resolve(initial, target.id)
        .expect("resolve CAD target")
        .expect("CAD target revision");
    let unrelated_cad = page
        .entities
        .iter()
        .find(|entity| entity.id != target.id)
        .expect("unrelated CAD entity");
    let unrelated_cad_revision = store
        .resolve(initial, unrelated_cad.id)
        .expect("resolve unrelated CAD entity")
        .expect("unrelated CAD revision");
    let original_dxf = fs::read(root.join("drawing.dxf")).expect("original DXF");
    let source_import = scriberig_domain_dxf::import_dxf(&original_dxf, drawing.drawing)
        .expect("import DXF external identities");
    let native_handle = source_import
        .provider_import
        .entity_identifier(target.id)
        .expect("DXF native handle provenance")
        .value
        .clone();
    let original_geo = fs::read(root.join("parcels.gpkg")).expect("original GeoPackage");
    let original_qgis = fs::read(root.join("project.qgs")).expect("original QGIS project");
    let cad_candidate = service
        .propose_dxf_layer_change(project, initial, target.id, labels_layer.id)
        .expect("propose exact CAD layer change");
    super::semantic_candidate_assertions::assert_persisted_semantic_change(
        &service,
        &cad_candidate,
        target.id,
        "entity.layer",
        &serde_json::json!(parcels_layer.id),
        &serde_json::json!(labels_layer.id),
    );
    assert_eq!(
        fs::read(root.join("drawing.dxf")).expect("live DXF before validation"),
        original_dxf
    );
    let cad_validated = service
        .validate_candidate_profile_cancellable(
            &cad_candidate.id,
            CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate CAD candidate");
    assert_eq!(cad_validated.validation, CandidateValidation::Passed);
    let cad_applied = service
        .apply_candidate(&cad_validated)
        .expect("apply validated CAD candidate");
    assert_eq!(
        store
            .change_provenance(cad_applied.successor_snapshot)
            .expect("CAD change provenance")
            .expect("CAD change")
            .semantic_operation
            .as_deref(),
        Some("cad.change_layer")
    );
    let reopened_drawing = scriberig_domain_dxf::import_dxf(
        &fs::read(root.join("drawing.dxf")).expect("materialized DXF"),
        drawing.drawing,
    )
    .expect("reopen materialized DXF");
    let reopened_entity = reopened_drawing
        .drawing()
        .entities
        .iter()
        .find(|entity| entity.id == target.id)
        .expect("same semantic entity");
    assert_eq!(
        reopened_drawing
            .provider_import
            .entity_identifier(target.id)
            .expect("same native entity handle")
            .value,
        native_handle
    );
    assert_eq!(reopened_entity.layer, labels_layer.id);
    let changed_target_revision = store
        .resolve(cad_applied.successor_snapshot, target.id)
        .expect("resolve CAD target after apply")
        .expect("CAD target revision after apply");
    assert_ne!(changed_target_revision, target_revision);
    assert_eq!(
        store
            .resolve(cad_applied.successor_snapshot, unrelated_cad.id)
            .expect("resolve unrelated CAD entity after apply"),
        Some(unrelated_cad_revision)
    );
    assert_eq!(
        fs::read(root.join("parcels.gpkg")).expect("GeoPackage after CAD edit"),
        original_geo,
        "CAD materialization must preserve unrelated rich artifacts"
    );

    let geo_layer = entities
        .iter()
        .find(|entity| entity.kind == "geo.layer" && entity.relative_path == "parcels.gpkg")
        .expect("GeoPackage layer");
    let before_geo = service
        .geopackage_feature_page(
            project,
            "parcels.gpkg",
            cad_applied.successor_snapshot,
            geo_layer.entity,
            0,
            2,
        )
        .expect("features after CAD edit");
    let target_feature = before_geo
        .features
        .iter()
        .find(|feature| feature.native_id.as_deref() == Some("124"))
        .expect("target parcel");
    let neighbor = before_geo
        .features
        .iter()
        .find(|feature| feature.native_id.as_deref() == Some("125"))
        .expect("neighbor parcel");
    let feature_revision = store
        .resolve(cad_applied.successor_snapshot, target_feature.id)
        .expect("resolve target parcel")
        .expect("target parcel revision");
    let neighbor_revision = store
        .resolve(cad_applied.successor_snapshot, neighbor.id)
        .expect("resolve neighbor parcel")
        .expect("neighbor parcel revision");
    let geo_candidate = service
        .propose_geopackage_feature_attribute(&GeoPackageFeatureAttributeEdit {
            project,
            base_snapshot: cad_applied.successor_snapshot,
            path: "parcels.gpkg".to_owned(),
            layer: geo_layer.entity,
            feature: target_feature.id,
            field: "area".to_owned(),
            value: GeoValue::Number("15.5".to_owned()),
        })
        .expect("propose exact feature attribute change");
    assert_eq!(
        fs::read(root.join("parcels.gpkg")).expect("live GeoPackage before validation"),
        original_geo
    );
    let geo_validated = service
        .validate_candidate_profile_cancellable(
            &geo_candidate.id,
            CandidateValidationProfile::GeoPackagePackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate GeoPackage candidate");
    assert_eq!(geo_validated.validation, CandidateValidation::Passed);
    let geo_applied = service
        .apply_candidate(&geo_validated)
        .expect("apply validated GeoPackage candidate");
    assert_eq!(
        store
            .change_provenance(geo_applied.successor_snapshot)
            .expect("GeoPackage change provenance")
            .expect("GeoPackage change")
            .semantic_operation
            .as_deref(),
        Some("geo.set_attribute")
    );
    let materialized_geo = fs::read(root.join("parcels.gpkg")).expect("materialized GeoPackage");
    assert_ne!(materialized_geo, original_geo);
    let dataset = entities
        .iter()
        .find(|entity| entity.kind == "geo.dataset" && entity.relative_path == "parcels.gpkg")
        .expect("GeoPackage dataset");
    let reopened_geo = GeoPackageReader::open(&materialized_geo, dataset.entity)
        .expect("reopen materialized GeoPackage");
    let reopened_feature = reopened_geo
        .feature_page(geo_layer.entity, 0, 2)
        .expect("read materialized GeoPackage features")
        .features
        .into_iter()
        .find(|feature| feature.id == target_feature.id)
        .expect("reopen edited feature");
    assert_eq!(
        reopened_feature.attributes.get("area"),
        Some(&GeoValue::Number("15.5".to_owned()))
    );
    let after_geo = service
        .geopackage_feature_page(
            project,
            "parcels.gpkg",
            geo_applied.successor_snapshot,
            geo_layer.entity,
            0,
            2,
        )
        .expect("features after GeoPackage edit");
    let edited_feature = after_geo
        .features
        .iter()
        .find(|feature| feature.id == target_feature.id)
        .expect("edited feature identity");
    let unchanged_neighbor = after_geo
        .features
        .iter()
        .find(|feature| feature.id == neighbor.id)
        .expect("unchanged neighbor identity");
    assert_eq!(
        edited_feature.attributes.get("area"),
        Some(&GeoValue::Number("15.5".to_owned()))
    );
    assert_eq!(unchanged_neighbor, neighbor);
    assert_ne!(
        store
            .resolve(geo_applied.successor_snapshot, target_feature.id)
            .expect("resolve updated feature"),
        Some(feature_revision)
    );
    assert_eq!(
        store
            .resolve(geo_applied.successor_snapshot, neighbor.id)
            .expect("resolve unchanged neighbor"),
        Some(neighbor_revision)
    );
    assert_eq!(
        store
            .resolve(geo_applied.successor_snapshot, target.id)
            .expect("resolve CAD entity after GeoPackage edit"),
        store
            .resolve(cad_applied.successor_snapshot, target.id)
            .expect("resolve CAD entity before GeoPackage edit")
    );
    assert_eq!(
        store
            .resolve(geo_applied.successor_snapshot, unrelated_cad.id)
            .expect("resolve unrelated CAD entity after GeoPackage edit"),
        Some(unrelated_cad_revision)
    );
    assert_ne!(geo_applied.successor_snapshot, initial);
    assert_ne!(
        geo_applied.successor_snapshot,
        cad_applied.successor_snapshot
    );
    assert_eq!(
        fs::read(root.join("project.qgs")).expect("QGIS project after dataset edit"),
        original_qgis,
        "editing a dataset alone must not rewrite the QGIS project"
    );

    let qgis = service
        .qgis_project(project, "project.qgs", geo_applied.successor_snapshot)
        .expect("QGIS references after GeoPackage materialization");
    assert!(
        qgis.project
            .layers
            .iter()
            .any(|layer| { layer.name == "Parcels" && layer.layer == Some(geo_layer.entity) })
    );
    drop(service);
    drop(store);

    let reopened_store = Arc::new(SqliteStore::open(&database).expect("reopen database"));
    let reopened_service = ProjectService::new(Arc::clone(&reopened_store));
    let reopened_workspace = reopened_service
        .workspace(project)
        .expect("reopen project workspace");
    assert_eq!(reopened_workspace.snapshot, geo_applied.successor_snapshot);
    assert_eq!(
        reopened_workspace.root_entity,
        initial_workspace.root_entity
    );
    for entity in &entities {
        assert!(
            reopened_store
                .resolve(reopened_workspace.snapshot, entity.entity)
                .expect("resolve persisted semantic handle")
                .is_some(),
            "entity {} of kind {} must resolve after reopen",
            entity.entity,
            entity.kind
        );
    }
    let persisted_files = reopened_store
        .project_file_descriptors(project)
        .expect("persisted source descriptors");
    for file in &persisted_files {
        assert_eq!(
            file.digest,
            *blake3::hash(
                &fs::read(root.join(&file.relative_path)).expect("reopened source artifact")
            )
            .as_bytes(),
            "source evidence for {} must survive application reopen",
            file.relative_path
        );
    }
    let persisted_geo = persisted_files
        .into_iter()
        .find(|file| file.relative_path == "parcels.gpkg")
        .expect("GeoPackage source evidence");
    assert_eq!(
        persisted_geo.digest,
        *blake3::hash(&materialized_geo).as_bytes()
    );
    assert_eq!(
        reopened_store
            .change_provenance(reopened_workspace.snapshot)
            .expect("persisted project history")
            .expect("GeoPackage edit provenance")
            .semantic_operation
            .as_deref(),
        Some("geo.set_attribute")
    );
    assert_eq!(
        reopened_store
            .change_provenance(cad_applied.successor_snapshot)
            .expect("persisted CAD history")
            .expect("CAD edit provenance")
            .semantic_operation
            .as_deref(),
        Some("cad.change_layer")
    );
    let reopened_qgis = reopened_service
        .qgis_project(project, "project.qgs", reopened_workspace.snapshot)
        .expect("reopen QGIS layer references");
    assert!(
        reopened_qgis
            .project
            .layers
            .iter()
            .any(|layer| { layer.name == "Parcels" && layer.layer == Some(geo_layer.entity) })
    );
    let reopened_features = reopened_service
        .geopackage_feature_page(
            project,
            "parcels.gpkg",
            reopened_workspace.snapshot,
            geo_layer.entity,
            0,
            2,
        )
        .expect("reopen edited feature page");
    assert_eq!(
        reopened_features.features[0].attributes.get("area"),
        Some(&GeoValue::Number("15.5".to_owned()))
    );
    drop(reopened_service);
    drop(reopened_store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove copied fixture");
}
