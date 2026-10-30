//! Verifies real imported `GeoPackage` reads across embedded and remote clients.

use std::fs;
use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityScope,
};
use scriberig_client::Client;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::{
    GeoAttributeEditRequest, GeoAttributeEditResponse, GeoValue as PublicGeoValue,
    ProtocolTransport,
};
use scriberig_server::InProcessTransport;

/// Creates a deterministic one-layer `GeoPackage` with two point features.
///
/// # Returns
/// Exact SQLite `GeoPackage` bytes suitable for isolated onboarding tests.
fn geopackage_bytes() -> Vec<u8> {
    let connection = rusqlite::Connection::open_in_memory().expect("open GeoPackage fixture");
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
                area REAL
            );
            INSERT INTO gpkg_contents
                (table_name,data_type,identifier,min_x,min_y,max_x,max_y,srs_id)
                VALUES ('parcels','features','Parcels',1,2,3,4,4326);
            INSERT INTO gpkg_geometry_columns
                VALUES ('parcels','geom','POINT',4326,0,0);",
        )
        .expect("create GeoPackage catalog");
    let mut geometry = b"GP\0\x01".to_vec();
    geometry.extend_from_slice(&4326_i32.to_le_bytes());
    geometry.push(1);
    geometry.extend_from_slice(&1_u32.to_le_bytes());
    geometry.extend_from_slice(&2.0_f64.to_le_bytes());
    geometry.extend_from_slice(&3.0_f64.to_le_bytes());
    connection
        .execute(
            "INSERT INTO parcels VALUES (124, ?1, 124, 10.5), (125, ?1, 125, 12.0)",
            rusqlite::params![geometry],
        )
        .expect("insert point features");
    connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize GeoPackage")
        .to_vec()
}

/// Provides exact Inspect authority for one integration-test project.
///
/// # Parameters
/// - `project`: Stable test project identity.
/// - `snapshot`: Exact imported snapshot.
///
/// Request context carrying the host-scoped Inspect grant.
///
/// # Returns
/// Application context for embedded app calls.
fn authorized_geo_context(project: ProjectId, snapshot: SnapshotId) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: std::collections::BTreeSet::from([CapabilityAction::Inspect]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    }
}

/// Verifies a remote attribute edit returns an exact-target shared candidate.
///
/// # Parameters
/// - `client`: Protocol client under test.
/// - `request`: Exact project, snapshot, feature, field, and replacement value.
///
/// # Returns
/// Candidate response after checking its exact operation target.
fn assert_attribute_candidate<T: ProtocolTransport>(
    client: &Client<T>,
    request: GeoAttributeEditRequest,
) -> GeoAttributeEditResponse {
    let expected = request.clone();
    let candidate = client
        .propose_geo_attribute_edit(request)
        .expect("remote feature attribute candidate");
    assert_eq!(candidate.base_snapshot, expected.snapshot);
    assert_eq!(candidate.layer, expected.layer);
    assert_eq!(candidate.feature, expected.feature);
    assert_eq!(candidate.field, expected.field);
    assert_eq!(candidate.path, expected.path);
    candidate
}

#[allow(
    clippy::too_many_lines,
    reason = "This transport integration test verifies one imported GeoPackage across catalog, feature paging, and remote client boundaries."
)]
/// Confirms embedded and remote clients read the same imported `GeoPackage` semantics.
#[test]
fn imported_geopackage_catalog_and_features_match_across_app_and_client() {
    let root =
        std::env::temp_dir().join(format!("scriberig-remote-geopackage-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("create project");
    fs::write(root.join("parcels.gpkg"), geopackage_bytes()).expect("persist GeoPackage");
    let database = root.with_extension("sqlite");
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(&database).expect("database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let imported = projects.import(scan.project.id).expect("import project");
    let project = scan.project.id;
    let layer = store
        .project_entities(project)
        .expect("layer ownership")
        .into_iter()
        .find(|entity| entity.kind == "geo.layer")
        .expect("semantic GeoLayer")
        .entity;
    let embedded = scriberig_app::ApplicationService::new(
        scriberig_app::StoreInspectionBackend::new(Arc::clone(&store)),
    )
    .with_project_service(projects.clone());
    let context = authorized_geo_context(project, imported.snapshot);
    let embedded_catalog = embedded
        .inspect_geopackage(&context, project, "parcels.gpkg", 0, 20)
        .expect("embedded GeoPackage catalog");
    let embedded_features = embedded
        .read_geopackage_features(&context, project, "parcels.gpkg", layer, 0, 20)
        .expect("embedded feature page");
    let grants = CapabilityContext::with_grants([Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: std::collections::BTreeSet::from([
            CapabilityAction::Inspect,
            CapabilityAction::Propose,
        ]),
        constraints: CapabilityConstraints::unrestricted(),
    }]);
    let remote_service = scriberig_app::ApplicationService::new(
        scriberig_app::StoreInspectionBackend::new(Arc::clone(&store)),
    )
    .with_project_service(projects.clone());
    let client = Client::new(InProcessTransport::new(remote_service).with_capabilities(grants));
    let remote_catalog = client
        .inspect_geopackage(
            project.to_string(),
            "parcels.gpkg",
            imported.snapshot.to_string(),
            0,
            20,
        )
        .expect("remote GeoPackage catalog");
    let remote_features = client
        .geo_features(
            project.to_string(),
            "parcels.gpkg",
            imported.snapshot.to_string(),
            layer.to_string(),
            0,
            20,
        )
        .expect("remote feature page");
    assert_eq!(
        remote_catalog.snapshot,
        embedded_catalog.snapshot.to_string()
    );
    assert_eq!(remote_catalog.layers.len(), embedded_catalog.layers.len());
    assert_eq!(remote_catalog.layers[0].id, layer.to_string());
    assert_eq!(remote_catalog.layers[0].feature_count, 2);
    assert_eq!(
        remote_features.features.len(),
        embedded_features.page.features.len()
    );
    assert_eq!(
        remote_features.features[0].id,
        embedded_features.page.features[0].id.to_string()
    );
    assert_eq!(
        remote_features.features[0].native_id.as_deref(),
        Some("124")
    );
    assert_eq!(
        remote_features.features[0].attributes["parcel_id"],
        PublicGeoValue::Number("124".into())
    );
    let attribute_edit = GeoAttributeEditRequest {
        project: project.to_string(),
        path: "parcels.gpkg".to_owned(),
        snapshot: imported.snapshot.to_string(),
        layer: layer.to_string(),
        feature: remote_features.features[0].id.clone(),
        field: "area".to_owned(),
        value: PublicGeoValue::Number("11.25".into()),
    };
    let remote_candidate = assert_attribute_candidate(&client, attribute_edit.clone());
    let persisted_candidate = projects
        .candidate(&remote_candidate.candidate)
        .expect("resolve remote candidate through embedded project service")
        .expect("remote candidate identity is canonical");
    assert_eq!(remote_candidate.candidate, persisted_candidate.id);
    assert_eq!(
        remote_candidate.base_snapshot,
        persisted_candidate.base_snapshot.to_string()
    );
    assert_eq!(persisted_candidate.project, project);
    assert_eq!(persisted_candidate.path, "parcels.gpkg");
    assert_eq!(
        persisted_candidate.semantic_operation.as_deref(),
        Some("geo.set_attribute")
    );

    drop(client);
    drop(embedded);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
