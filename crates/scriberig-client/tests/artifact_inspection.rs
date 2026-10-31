//! Verifies the semantic client uses the typed bounded artifact operation.

use scriberig_client::Client;
use scriberig_protocol::{
    ArtifactInspectResponse, ArtifactMemberSummary, ArtifactSummary, ProtocolTransport,
    ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
};

/// Returns one deterministic artifact page to the client.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Verifies artifact request fields and returns one typed fidelity summary.
    ///
    /// # Parameters
    /// - `request`: Typed protocol request from the reusable client.
    ///
    /// # Returns
    /// The exact typed response for the fixture artifact page.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(matches!(
            request.request,
            Request::ArtifactInspect(ref request)
                if request.project == "project-1" && request.offset == 20 && request.limit == 10
        ));
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::ArtifactInspect(ArtifactInspectResponse {
                project: "project-1".into(),
                artifacts: vec![ArtifactSummary {
                    id: "ent_artifact".into(),
                    logical_key: "xlsx:book.xlsx".into(),
                    kind: "xlsx".into(),
                    adapter: "xlsx".into(),
                    adapter_version: Some("1.2.3".into()),
                    scan_id: "ab".repeat(32),
                    baseline_digest: "ef".repeat(32),
                    observed_digest: Some("ef".repeat(32)),
                    synchronization_status: "in-sync".into(),
                    fidelity: "editable-with-opaque-preservation".into(),
                    opaque_preservation: "preserved".into(),
                    diagnostic: None,
                    references: Vec::new(),
                    members: vec![ArtifactMemberSummary {
                        key: "book.xlsx".into(),
                        project_path: Some("book.xlsx".into()),
                        container_part: None,
                        digest: "cd".repeat(32),
                    }],
                    member_count: 1,
                }],
                total: 23,
                next_offset: None,
            }),
        }
    }
}

/// Requests one artifact page through the reusable client API.
#[test]
fn client_returns_typed_artifact_page() {
    let page = Client::new(FixtureTransport)
        .inspect_artifacts("project-1", 20, 10)
        .expect("artifact page");
    assert_eq!(page.total, 23);
    assert_eq!(page.project, "project-1");
    assert_eq!(
        page.artifacts[0].fidelity,
        "editable-with-opaque-preservation"
    );
    assert_eq!(page.artifacts[0].opaque_preservation, "preserved");
}
