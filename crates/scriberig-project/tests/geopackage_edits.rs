//! Verifies `GeoPackage` geometry changes use candidate validation and shared apply.

use std::fs;
use std::sync::Arc;

use rusqlite::Connection;
use scriberig_domain_geo::{Coordinate, Geometry};
use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, ProjectService, ProjectSourceChange,
    ProjectSourceChangeKind,
};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

#[path = "geopackage_edits/attribute.rs"]
mod attribute;
#[path = "geopackage_edits/failure.rs"]
mod failure;

/// Creates a two-feature `GeoPackage` with typed point metadata.
///
/// # Returns
/// Serialized database bytes for parcels 124 and 125.
fn geopackage_fixture() -> Vec<u8> {
    let connection = Connection::open_in_memory().expect("fixture database");
    connection
        .execute_batch(
            "CREATE TABLE gpkg_contents (
                table_name TEXT PRIMARY KEY, data_type TEXT NOT NULL,
                identifier TEXT, description TEXT DEFAULT '',
                last_change DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER
            );
            CREATE TABLE gpkg_geometry_columns (
                table_name TEXT, column_name TEXT, geometry_type_name TEXT,
                srs_id INTEGER, z TINYINT, m TINYINT
            );
            CREATE TABLE gpkg_spatial_ref_sys (
                srs_name TEXT, srs_id INTEGER PRIMARY KEY, organization TEXT,
                organization_coordsys_id INTEGER, definition TEXT, description TEXT
            );
            INSERT INTO gpkg_spatial_ref_sys VALUES
                ('WGS 84', 4326, 'EPSG', 4326, 'EPSG:4326', '');
            CREATE TABLE parcels (
                fid INTEGER PRIMARY KEY, geom BLOB, parcel_id INTEGER NOT NULL,
                area REAL, label TEXT
            );
            INSERT INTO gpkg_contents
                (table_name,data_type,identifier,min_x,min_y,max_x,max_y,srs_id)
                VALUES ('parcels','features','Parcels',1,2,3,4,4326);
            INSERT INTO gpkg_geometry_columns
                VALUES ('parcels','geom','POINT',4326,0,0);",
        )
        .expect("package catalog");
    let point = point_blob(4326, 2.0, 3.0);
    connection
        .execute(
            "INSERT INTO parcels VALUES (124, ?1, 124, 10.5, 'Parcel 124'),
                (125, ?1, 125, 12.0, 'Parcel 125')",
            rusqlite::params![point],
        )
        .expect("parcel features");
    connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize package")
        .to_vec()
}

/// Encodes one little-endian point geometry blob.
///
/// # Parameters
/// - `srs_id`: Spatial reference identifier.
/// - `x`: Point x ordinate.
/// - `y`: Point y ordinate.
///
/// # Returns
/// `GeoPackage` geometry header followed by little-endian point WKB.
fn point_blob(srs_id: i32, x: f64, y: f64) -> Vec<u8> {
    let mut bytes = b"GP\0\x01".to_vec();
    bytes.extend_from_slice(&srs_id.to_le_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&x.to_le_bytes());
    bytes.extend_from_slice(&y.to_le_bytes());
    bytes
}

/// Creates and applies a validated one-feature geometry candidate end to end.
///
/// The single integration flow deliberately checks the live source, candidate
/// gate, successor snapshot, unrelated row, digest, provider record, and stale
/// base so those guarantees stay tied to one real project lifecycle.
#[allow(clippy::too_many_lines)]
#[test]
fn geometry_candidate_updates_one_feature_after_validation_only() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-geopackage-edit-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    fs::write(root.join("parcels.gpkg"), geopackage_fixture()).expect("write GeoPackage fixture");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    service.import(project).expect("import project");
    let workspace = service.workspace(project).expect("workspace");
    let layer = store
        .project_entities(project)
        .expect("owned entities")
        .into_iter()
        .find(|entity| entity.kind == "geo.layer")
        .expect("parcel layer");
    let before = fs::read(root.join("parcels.gpkg")).expect("original package");
    let original_page = service
        .geopackage_feature_page(
            project,
            "parcels.gpkg",
            workspace.snapshot,
            layer.entity,
            0,
            2,
        )
        .expect("read source features");
    let target = &original_page.features[0];
    let neighbor = original_page.features[1].clone();
    let original_target_revision = feature_revision_id(&store, workspace.snapshot, target.id);
    let original_neighbor_revision = feature_revision_id(&store, workspace.snapshot, neighbor.id);
    let mut invalid_bytes = before.clone();
    invalid_bytes[0] = 0;
    let invalid_candidate = service
        .propose_project_change(
            project,
            workspace.snapshot,
            vec![ProjectSourceChange {
                path: "parcels.gpkg".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(invalid_bytes),
            }],
        )
        .expect("create invalid package candidate");
    let failed_validation = service
        .validate_candidate_profile_cancellable(
            &invalid_candidate.id,
            CandidateValidationProfile::GeoPackagePackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("persist failed package validation");
    assert!(matches!(
        failed_validation.validation,
        CandidateValidation::Failed { .. }
    ));
    assert!(service.apply_candidate(&failed_validation).is_err());
    assert_eq!(
        fs::read(root.join("parcels.gpkg")).expect("live source"),
        before
    );
    let replacement = Geometry::Point(Coordinate {
        x: "2.5".to_owned(),
        y: "3.5".to_owned(),
        z: None,
        measure: None,
    });

    let candidate = service
        .propose_geopackage_feature_geometry(
            project,
            workspace.snapshot,
            "parcels.gpkg",
            layer.entity,
            target.id,
            &replacement,
        )
        .expect("create geometry candidate");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("geo.replace_geometry")
    );
    assert_eq!(candidate.semantic_changes[0].target, target.id);
    assert_eq!(candidate.semantic_changes[0].property, "feature.geometry");
    assert_eq!(
        candidate.semantic_changes[0].before,
        serde_json::to_value(&target.geometry).expect("prior geometry JSON")
    );
    assert_eq!(
        candidate.semantic_changes[0].after,
        serde_json::to_value(&replacement).expect("proposed geometry JSON")
    );
    assert_eq!(
        service
            .candidate(&candidate.id)
            .expect("reload geometry candidate")
            .expect("durable geometry candidate")
            .semantic_operation
            .as_deref(),
        Some("geo.replace_geometry")
    );

    assert_eq!(
        fs::read(root.join("parcels.gpkg")).expect("live source"),
        before
    );
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::GeoPackagePackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate geometry candidate");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let external_bytes = [before.as_slice(), b"external"].concat();
    fs::write(root.join("parcels.gpkg"), &external_bytes).expect("simulate external change");
    let conflict = service
        .apply_candidate(&validated)
        .expect_err("changed live source must fail exact baseline preflight");
    assert_eq!(conflict.code, "filesystem_conflict");
    assert_eq!(
        fs::read(root.join("parcels.gpkg")).expect("external source remains"),
        external_bytes
    );
    fs::write(root.join("parcels.gpkg"), &before).expect("restore exact candidate baseline");
    let applied = service
        .apply_candidate(&validated)
        .expect("apply validated geometry candidate");
    let provenance = store
        .change_provenance(applied.successor_snapshot)
        .expect("read snapshot provenance")
        .expect("applied snapshot change");
    assert_eq!(provenance.change_id, candidate.id);
    assert_eq!(
        provenance.semantic_operation.as_deref(),
        Some("geo.replace_geometry")
    );
    let after = service
        .geopackage_feature_page(
            project,
            "parcels.gpkg",
            applied.successor_snapshot,
            layer.entity,
            0,
            2,
        )
        .expect("read applied features");

    assert_eq!(after.features[0].id, target.id);
    assert_eq!(after.features[0].geometry, Some(replacement));
    assert_eq!(after.features[1], neighbor);
    assert_ne!(
        feature_revision_id(&store, applied.successor_snapshot, target.id),
        original_target_revision,
        "geometry changes must create a new exact semantic feature revision"
    );
    assert_eq!(
        feature_revision_id(&store, applied.successor_snapshot, neighbor.id),
        original_neighbor_revision,
        "unmodified neighbor must retain its exact semantic feature revision"
    );
    let materialized = fs::read(root.join("parcels.gpkg")).expect("materialized source");
    assert_ne!(materialized, before);
    let source_record = store
        .project_files(project)
        .expect("updated source records")
        .into_iter()
        .find(|file| file.relative_path == "parcels.gpkg")
        .expect("materialized GeoPackage evidence");
    assert_eq!(
        source_record.digest,
        *blake3::hash(&materialized).as_bytes()
    );
    let dataset = store
        .project_entities(project)
        .expect("updated semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "geo.dataset")
        .expect("dataset summary");
    let dataset_revision = store
        .resolve(applied.successor_snapshot, dataset.entity)
        .expect("resolve dataset summary")
        .expect("dataset summary revision");
    let dataset_payload = store
        .get(dataset_revision)
        .expect("load dataset summary")
        .expect("dataset summary payload");
    let dataset_summary: serde_json::Value =
        serde_json::from_slice(dataset_payload.payload()).expect("decode dataset summary");
    assert_eq!(
        dataset_summary["provider"],
        "rusqlite-0.40.2-staged-geopackage-writer-v1"
    );
    let stale = service
        .propose_geopackage_feature_geometry(
            project,
            workspace.snapshot,
            "parcels.gpkg",
            layer.entity,
            target.id,
            &Geometry::Point(Coordinate {
                x: "2.75".to_owned(),
                y: "3.75".to_owned(),
                z: None,
                measure: None,
            }),
        )
        .expect_err("candidate from prior snapshot must be rejected");
    assert_eq!(stale.code, "stale_candidate");

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Resolves one exact feature revision from a requested project snapshot.
///
/// # Parameters
/// - `store`: SQLite-backed immutable revision and snapshot store.
/// - `snapshot`: Exact project snapshot containing the feature.
/// - `feature`: Stable semantic feature identity.
///
/// # Returns
/// Exact canonical feature revision identity.
fn feature_revision_id(
    store: &SqliteStore,
    snapshot: scriberig_core::SnapshotId,
    feature: scriberig_core::EntityId,
) -> scriberig_core::RevisionId {
    store
        .resolve(snapshot, feature)
        .expect("resolve exact feature revision")
        .expect("feature revision is present in exact snapshot")
}
