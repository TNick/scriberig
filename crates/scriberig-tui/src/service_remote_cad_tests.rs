//! Verifies remote CAD reads and candidates retain exact semantic scope.

use super::*;

/// Returns a remote drawing summary only when all identities match the request.
#[test]
fn remote_cad_drawing_read_preserves_snapshot_and_layers() {
    let project = scriberig_core::ProjectId::from_bytes([141; 16]).to_string();
    let snapshot = SnapshotId::new([142; 32]).to_string();
    let drawing = EntityId::from_bytes([143; 16]).to_string();
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "cad-drawing".to_owned(),
            response: Response::CadDrawingInspect(scriberig_protocol::CadDrawingInspectResponse {
                project: project.clone(),
                snapshot: snapshot.clone(),
                drawing: drawing.clone(),
                units: "meters".to_owned(),
                coordinate_reference: None,
                layers: vec![scriberig_protocol::CadLayerSummary {
                    id: "layer-1".to_owned(),
                    name: "PARCELS".to_owned(),
                    visible: true,
                    locked: false,
                }],
                entity_count: 1,
                complete: true,
                diagnostics: Vec::new(),
            }),
        },
    }));
    let result = adapter
        .inspect_cad_drawing(
            &ApplicationContext::empty(),
            &scriberig_protocol::CadDrawingInspectRequest {
                project: project.clone(),
                snapshot: snapshot.clone(),
                drawing: drawing.clone(),
            },
        )
        .expect("remote exact-snapshot CAD drawing");
    assert_eq!(result.project, project);
    assert_eq!(result.snapshot, snapshot);
    assert_eq!(result.drawing, drawing);
    assert_eq!(result.layers[0].name, "PARCELS");
}

/// Refuses a remote CAD entity page that belongs to another snapshot.
#[test]
fn remote_cad_entity_page_rejects_identity_mismatch() {
    let project = scriberig_core::ProjectId::from_bytes([144; 16]).to_string();
    let snapshot = SnapshotId::new([145; 32]).to_string();
    let drawing = EntityId::from_bytes([146; 16]).to_string();
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "cad-entities".to_owned(),
            response: Response::CadEntities(scriberig_protocol::CadEntitiesResponse {
                project: project.clone(),
                snapshot: SnapshotId::new([147; 32]).to_string(),
                drawing: drawing.clone(),
                entities: Vec::new(),
                total: 0,
                has_more: false,
            }),
        },
    }));
    let result = adapter.inspect_cad_entities(
        &ApplicationContext::empty(),
        &scriberig_protocol::CadEntitiesRequest {
            project,
            snapshot,
            drawing,
            offset: 0,
            limit: 100,
        },
    );
    assert!(result.is_err());
}

/// Preserves a remote exact-base layer candidate for common Changes review.
#[test]
fn remote_cad_layer_candidate_keeps_exact_base_and_handle() {
    let project = scriberig_core::ProjectId::from_bytes([148; 16]).to_string();
    let snapshot = SnapshotId::new([149; 32]).to_string();
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "cad-layer".to_owned(),
            response: Response::CadLayerChange(scriberig_protocol::CadLayerChangeResponse {
                candidate: "cad-candidate-1".to_owned(),
                project: project.clone(),
                base_snapshot: snapshot.clone(),
                path: "drawing.dxf".to_owned(),
                validation: scriberig_protocol::CadCandidateValidation::Pending,
            }),
        },
    }));
    let candidate = adapter
        .propose_cad_layer_change(
            &ApplicationContext::empty(),
            &scriberig_protocol::CadLayerChangeRequest {
                project: project.clone(),
                snapshot: snapshot.clone(),
                entity: "entity-1".to_owned(),
                destination_layer: "layer-2".to_owned(),
            },
        )
        .expect("remote CAD layer candidate");
    assert_eq!(candidate.candidate, "cad-candidate-1");
    assert_eq!(candidate.project, project);
    assert_eq!(candidate.base_snapshot, snapshot);
}
