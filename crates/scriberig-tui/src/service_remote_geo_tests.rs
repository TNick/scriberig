//! Verifies remote geospatial reads and candidates retain their exact request scope.

use super::*;

/// Rejects a remote feature response whose snapshot differs from the exact request.
#[test]
fn remote_geo_feature_read_rejects_snapshot_mismatch() {
    let project = scriberig_core::ProjectId::from_bytes([131; 16]);
    let snapshot = SnapshotId::new([132; 32]);
    let layer = EntityId::from_bytes([133; 16]);
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "geo-read".to_owned(),
            response: Response::GeoFeatures(scriberig_protocol::GeoFeaturesResponse {
                project: project.to_string(),
                snapshot: SnapshotId::new([134; 32]).to_string(),
                layer: layer.to_string(),
                features: Vec::new(),
                has_more: false,
            }),
        },
    }));
    assert!(
        adapter
            .read_geopackage_features(
                &ApplicationContext::default(),
                project,
                "parcels.gpkg",
                snapshot,
                layer,
                0,
            )
            .is_err()
    );
}

/// Maps a remote geometry candidate only when all exact target identities match.
#[test]
fn remote_geo_geometry_candidate_preserves_target_and_base() {
    let project = scriberig_core::ProjectId::from_bytes([135; 16]);
    let snapshot = SnapshotId::new([136; 32]);
    let layer = EntityId::from_bytes([137; 16]);
    let feature = EntityId::from_bytes([138; 16]);
    let response = scriberig_protocol::GeoGeometryEditResponse {
        candidate: "geo-candidate-1".to_owned(),
        project: project.to_string(),
        base_snapshot: snapshot.to_string(),
        layer: layer.to_string(),
        feature: feature.to_string(),
        path: "parcels.gpkg".to_owned(),
    };
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "geo-edit".to_owned(),
            response: Response::GeoGeometryEdit(response),
        },
    }));
    let request = scriberig_protocol::GeoGeometryEditRequest {
        project: project.to_string(),
        path: "parcels.gpkg".to_owned(),
        snapshot: snapshot.to_string(),
        layer: layer.to_string(),
        feature: feature.to_string(),
        geometry: scriberig_protocol::GeoGeometry::Point(scriberig_protocol::GeoCoordinate {
            x: "1.00".to_owned(),
            y: "2.00".to_owned(),
            z: None,
            measure: None,
        }),
    };
    let candidate = adapter
        .propose_geo_geometry_edit(&ApplicationContext::default(), &request)
        .expect("remote exact-base geometry candidate");
    assert_eq!(candidate.id, "geo-candidate-1");
    assert_eq!(candidate.project, project);
    assert_eq!(candidate.base_snapshot, snapshot);
    assert_eq!(candidate.layer, layer);
    assert_eq!(candidate.feature, feature);
}

/// Preserves project, base snapshot, and reference identities for remote QGIS candidates.
#[test]
fn remote_qgis_rename_candidate_preserves_exact_scope() {
    let project = scriberig_core::ProjectId::from_bytes([139; 16]);
    let snapshot = SnapshotId::new([140; 32]);
    let reference = EntityId::from_bytes([141; 16]);
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "qgis-rename".to_owned(),
            response: Response::QgisLayerRename(scriberig_protocol::QgisLayerRenameResponse {
                candidate: "qgis-candidate-1".to_owned(),
                project: project.to_string(),
                base_snapshot: snapshot.to_string(),
                reference: reference.to_string(),
                path: "maps/plan.qgs".to_owned(),
            }),
        },
    }));
    let request = scriberig_protocol::QgisLayerRenameRequest {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        reference: reference.to_string(),
        new_name: "Parcels Updated".to_owned(),
    };
    let candidate = adapter
        .propose_qgis_layer_rename(&ApplicationContext::default(), &request)
        .expect("remote exact-base QGIS rename candidate");
    assert_eq!(candidate.candidate, "qgis-candidate-1");
    assert_eq!(candidate.project, project.to_string());
    assert_eq!(candidate.base_snapshot, snapshot.to_string());
    assert_eq!(candidate.reference, reference.to_string());
    assert_eq!(candidate.path, "maps/plan.qgs");
}

/// Preserves project, snapshot, path, and semantic order through remote QGIS candidates.
#[test]
fn remote_qgis_layer_order_candidate_preserves_exact_scope() {
    let project = scriberig_core::ProjectId::from_bytes([142; 16]);
    let snapshot = SnapshotId::new([143; 32]);
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "qgis-order".to_owned(),
            response: Response::QgisLayerOrder(scriberig_protocol::QgisLayerOrderResponse {
                candidate: "qgis-order-candidate-1".to_owned(),
                project: project.to_string(),
                base_snapshot: snapshot.to_string(),
                path: "maps/plan.qgs".to_owned(),
            }),
        },
    }));
    let request = scriberig_protocol::QgisLayerOrderRequest {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        path: "maps/plan.qgs".to_owned(),
        ordered_references: vec![
            EntityId::from_bytes([145; 16]).to_string(),
            EntityId::from_bytes([144; 16]).to_string(),
        ],
    };

    let candidate = adapter
        .propose_qgis_layer_order(&ApplicationContext::default(), &request)
        .expect("remote exact-base QGIS order candidate");
    assert_eq!(candidate.candidate, "qgis-order-candidate-1");
    assert_eq!(candidate.project, project.to_string());
    assert_eq!(candidate.base_snapshot, snapshot.to_string());
    assert_eq!(candidate.path, "maps/plan.qgs");
}

/// Routes personal source-style writes through the shared remote protocol contract.
#[test]
fn remote_personal_source_style_write_preserves_authoritative_value() {
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "source-style".to_owned(),
            response: Response::ProjectionStyleSetting(
                scriberig_protocol::ProjectionStyleSettingResponse {
                    style: Some(b"indent=tabs,indent-width=2,blank-lines=0".to_vec()),
                    revision: Some(7),
                },
            ),
        },
    }));
    let request = scriberig_protocol::ProjectionStyleSettingRequest {
        scope: scriberig_protocol::ProjectionStyleScope::User,
        projection_kind: "rust".to_owned(),
        expected_revision: None,
        style: Some(b" indent = tabs, indent-width = 2, blank-lines = 0 ".to_vec()),
    };
    let setting = adapter
        .set_projection_style_preference(&ApplicationContext::default(), &request)
        .expect("remote personal style preference");
    assert_eq!(
        setting.style.as_deref(),
        Some(b"indent=tabs,indent-width=2,blank-lines=0".as_slice())
    );
    assert_eq!(setting.revision, Some(7));
}
