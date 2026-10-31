//! Verifies the semantic client emits exact-snapshot typed CAD edit requests.

use scriberig_client::Client;
use scriberig_protocol::{
    CadBounds2D, CadBoundsQueryResponse, CadCandidateValidation, CadCoordinate, CadDrawingSpace,
    CadLayerChangeResponse, CadLineEditResponse, CadTextFindResponse, ProtocolTransport,
    ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
};

/// Confirms the CAD line request and returns a pending candidate.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Verifies each request anchor and returns its typed candidate response.
    ///
    /// # Parameters
    /// - `request`: Typed protocol envelope emitted by the semantic client.
    ///
    /// # Returns
    /// Typed response envelope for either tested candidate operation.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        let response = match request.request {
            Request::CadLineEdit(request) => {
                assert_eq!(request.project, "project-1");
                assert_eq!(request.snapshot, "snapshot-2");
                assert_eq!(request.entity, "line-3");
                assert_eq!(request.start.x, "1");
                Response::CadLineEdit(CadLineEditResponse {
                    candidate: "candidate-4".into(),
                    project: "project-1".into(),
                    base_snapshot: "snapshot-2".into(),
                    path: "drawing.dxf".into(),
                    validation: CadCandidateValidation::Pending,
                })
            }
            Request::CadLayerChange(request) => {
                assert_eq!(request.project, "project-1");
                assert_eq!(request.snapshot, "snapshot-2");
                assert_eq!(request.entity, "line-3");
                assert_eq!(request.destination_layer, "layer-5");
                Response::CadLayerChange(CadLayerChangeResponse {
                    candidate: "candidate-6".into(),
                    project: "project-1".into(),
                    base_snapshot: "snapshot-2".into(),
                    path: "drawing.dxf".into(),
                    validation: CadCandidateValidation::Pending,
                })
            }
            Request::CadBoundsQuery(request) => {
                assert_eq!(request.project, "project-1");
                assert_eq!(request.snapshot, "snapshot-2");
                assert_eq!(request.drawing, "drawing-3");
                assert_eq!(request.bounds.min_x, "0");
                Response::CadBoundsQuery(CadBoundsQueryResponse {
                    project: request.project,
                    snapshot: request.snapshot,
                    drawing: request.drawing,
                    space: CadDrawingSpace::Model,
                    bounds: request.bounds,
                    entities: Vec::new(),
                    total: 1,
                    has_more: false,
                    unbounded_entities: 0,
                    complete: true,
                })
            }
            Request::CadTextFind(request) => {
                assert_eq!(request.project, "project-1");
                assert_eq!(request.snapshot, "snapshot-2");
                assert_eq!(request.drawing, "drawing-3");
                assert_eq!(request.query, "parcel 124");
                Response::CadTextFind(CadTextFindResponse {
                    project: request.project,
                    snapshot: request.snapshot,
                    drawing: request.drawing,
                    matches: Vec::new(),
                    total: 0,
                    has_more: false,
                })
            }
            request => panic!("unexpected request: {request:?}"),
        };
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response,
        }
    }
}

/// Returns the candidate handle from the remote semantic CAD operation.
#[test]
fn client_returns_typed_cad_line_candidate() {
    let candidate = Client::new(FixtureTransport)
        .propose_cad_line_edit(
            "project-1",
            "snapshot-2",
            "line-3",
            CadCoordinate {
                x: "1".into(),
                y: "2".into(),
                z: "0".into(),
            },
            CadCoordinate {
                x: "3".into(),
                y: "4".into(),
                z: "0".into(),
            },
        )
        .expect("CAD candidate");
    assert_eq!(candidate.candidate, "candidate-4");
}

/// Returns the candidate handle from the remote semantic CAD layer operation.
#[test]
fn client_returns_typed_cad_layer_candidate() {
    let candidate = Client::new(FixtureTransport)
        .propose_cad_layer_change("project-1", "snapshot-2", "line-3", "layer-5")
        .expect("CAD layer candidate");
    assert_eq!(candidate.candidate, "candidate-6");
}

/// Reads a typed spatial query through the remote semantic client.
#[test]
fn client_returns_typed_cad_bounds_query() {
    let response = Client::new(FixtureTransport)
        .cad_entities_intersecting_bounds(scriberig_protocol::CadBoundsQueryRequest {
            project: "project-1".into(),
            snapshot: "snapshot-2".into(),
            drawing: "drawing-3".into(),
            space: CadDrawingSpace::Model,
            bounds: CadBounds2D {
                min_x: "0".into(),
                min_y: "0".into(),
                max_x: "1".into(),
                max_y: "1".into(),
            },
            offset: 0,
            limit: 10,
        })
        .expect("CAD bounds query");
    assert_eq!(response.total, 1);
    assert!(response.complete);
}

/// Preserves exact CAD search scope and bounded page parameters through the client.
#[test]
fn client_returns_typed_cad_text_search() {
    let response = Client::new(FixtureTransport)
        .find_cad_text("project-1", "snapshot-2", "drawing-3", "parcel 124", 0, 10)
        .expect("CAD text search");
    assert_eq!(response.drawing, "drawing-3");
    assert_eq!(response.matches.len(), 0);
    assert!(!response.has_more);
}
