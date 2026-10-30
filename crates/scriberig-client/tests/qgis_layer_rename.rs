//! Verifies the client sends exact QGIS layer rename requests.

use scriberig_client::Client;
use scriberig_protocol::{
    ProtocolTransport, ProtocolVersion, QgisLayerRenameRequest, QgisLayerRenameResponse, Request,
    RequestEnvelope, Response, ResponseEnvelope,
};

/// Confirms the semantic reference identity and returns candidate metadata.
struct RenameTransport;

impl ProtocolTransport for RenameTransport {
    /// Checks the QGIS rename request and returns a candidate response.
    ///
    /// # Parameters
    /// - `request`: Typed client request envelope.
    ///
    /// # Returns
    /// Protocol response with the exact candidate handle.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(
            matches!(request.request, Request::QgisLayerRename(ref request)
            if request.project == "project-1"
                && request.snapshot == "snapshot-2"
                && request.reference == "reference-3"
                && request.new_name == "Lots")
        );
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::QgisLayerRename(QgisLayerRenameResponse {
                candidate: "candidate-4".into(),
                project: "project-1".into(),
                base_snapshot: "snapshot-2".into(),
                reference: "reference-3".into(),
                path: "project.qgs".into(),
            }),
        }
    }
}

/// Returns the candidate handle from a typed QGIS layer rename request.
#[test]
fn client_proposes_qgis_layer_rename() {
    let candidate = Client::new(RenameTransport)
        .propose_qgis_layer_rename(QgisLayerRenameRequest {
            project: "project-1".into(),
            snapshot: "snapshot-2".into(),
            reference: "reference-3".into(),
            new_name: "Lots".into(),
        })
        .expect("QGIS rename candidate");
    assert_eq!(candidate.candidate, "candidate-4");
    assert_eq!(candidate.reference, "reference-3");
}
