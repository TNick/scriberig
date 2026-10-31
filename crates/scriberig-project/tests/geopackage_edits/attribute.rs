//! Verifies typed `GeoPackage` attribute candidates use shared validation and apply.

use std::fs;
use std::sync::Arc;

use scriberig_domain_geo::GeoValue;
use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, GeoPackageFeatureAttributeEdit, ProjectService,
};
use scriberig_store_sqlite::SqliteStore;

use super::geopackage_fixture;

/// Stages, validates, and applies a type-safe feature attribute candidate.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "This integration test verifies the full attribute candidate lifecycle and source isolation."
)]
/// Updates one typed feature field after validation and preserves neighbors.
fn attribute_candidate_updates_one_field_after_validation_only() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-geopackage-attribute-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let package = geopackage_fixture();
    fs::write(root.join("parcels.gpkg"), &package).expect("write GeoPackage fixture");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let imported = service.import(project).expect("import project");
    let layer = store
        .project_entities(project)
        .expect("owned entities")
        .into_iter()
        .find(|entity| entity.kind == "geo.layer")
        .expect("parcel layer")
        .entity;
    let page = service
        .geopackage_feature_page(project, "parcels.gpkg", imported.snapshot, layer, 0, 2)
        .expect("read exact feature page");
    let target = &page.features[0];
    let neighbor = page.features[1].clone();

    let candidate = service
        .propose_geopackage_feature_attribute(&GeoPackageFeatureAttributeEdit {
            project,
            base_snapshot: imported.snapshot,
            path: "parcels.gpkg".into(),
            layer,
            feature: target.id,
            field: "area".into(),
            value: GeoValue::Number("11.75".into()),
        })
        .expect("schema-valid area candidate");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("geo.set_attribute")
    );
    assert_eq!(candidate.semantic_changes[0].target, target.id);
    assert_eq!(
        candidate.semantic_changes[0].property,
        "feature.attribute.area"
    );
    assert_eq!(
        candidate.semantic_changes[0].before,
        serde_json::json!({"Number": "10.5"})
    );
    assert_eq!(
        candidate.semantic_changes[0].after,
        serde_json::json!({"Number": "11.75"})
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
        fs::read(root.join("parcels.gpkg")).expect("live source"),
        package
    );
    assert!(
        service
            .propose_geopackage_feature_attribute(&GeoPackageFeatureAttributeEdit {
                project,
                base_snapshot: imported.snapshot,
                path: "parcels.gpkg".into(),
                layer,
                feature: target.id,
                field: "area".into(),
                value: GeoValue::Text("large".into()),
            },)
            .is_err()
    );

    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::GeoPackagePackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate attribute candidate");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("apply validated attribute candidate");
    let after = service
        .geopackage_feature_page(
            project,
            "parcels.gpkg",
            applied.successor_snapshot,
            layer,
            0,
            2,
        )
        .expect("read changed snapshot");
    assert_eq!(after.features[0].id, target.id);
    assert_eq!(
        after.features[0].attributes.get("area"),
        Some(&GeoValue::Number("11.75".into()))
    );
    assert_eq!(after.features[1], neighbor);
    assert_ne!(
        fs::read(root.join("parcels.gpkg")).expect("materialized source"),
        package
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
