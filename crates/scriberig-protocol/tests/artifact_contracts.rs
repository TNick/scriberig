//! Verifies typed artifact inspection requests and summaries round trip.

use scriberig_protocol::{
    ArtifactInspectRequest, ArtifactInspectResponse, ArtifactMemberSummary, ArtifactMembersRequest,
    ArtifactMembersResponse, ArtifactSummary, ProtocolVersion, Request, RequestEnvelope, Response,
    ResponseEnvelope,
};

/// Preserves bounded artifact request and exact-member summary fields on the wire.
#[test]
fn artifact_inspection_envelope_round_trips() {
    let request = RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "artifact-1".into(),
        request: Request::ArtifactInspect(ArtifactInspectRequest {
            project: "prj_0123456789abcdef0123456789abcdef".into(),
            offset: 0,
            limit: 25,
        }),
    };
    let request_bytes = serde_json::to_vec(&request).expect("encode request");
    let decoded_request: RequestEnvelope =
        serde_json::from_slice(&request_bytes).expect("decode request");
    assert_eq!(decoded_request, request);

    let response = ResponseEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "artifact-1".into(),
        response: Response::ArtifactInspect(ArtifactInspectResponse {
            project: "prj_0123456789abcdef0123456789abcdef".into(),
            artifacts: vec![ArtifactSummary {
                id: "ent_0123456789abcdef0123456789abcdef".into(),
                logical_key: "shapefile:roads".into(),
                kind: "shapefile".into(),
                adapter: "binary".into(),
                adapter_version: None,
                scan_id: "ab".repeat(32),
                baseline_digest: "ef".repeat(32),
                observed_digest: Some("ef".repeat(32)),
                synchronization_status: "in-sync".into(),
                fidelity: "read-only-derived".into(),
                opaque_preservation: "unproven".into(),
                diagnostic: Some("No writer is registered".into()),
                references: Vec::new(),
                members: vec![ArtifactMemberSummary {
                    key: "roads.shp".into(),
                    project_path: Some("roads.shp".into()),
                    container_part: None,
                    digest: "cd".repeat(32),
                }],
                member_count: 1,
            }],
            total: 1,
            next_offset: None,
        }),
    };
    let response_bytes = serde_json::to_vec(&response).expect("encode response");
    let decoded_response: ResponseEnvelope =
        serde_json::from_slice(&response_bytes).expect("decode response");
    assert_eq!(decoded_response, response);
}

/// Preserves exact member paging request and response fields on the wire.
#[test]
fn artifact_member_page_envelope_round_trips() {
    let request = RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "artifact-members-1".into(),
        request: Request::ArtifactMembers(ArtifactMembersRequest {
            project: "prj_0123456789abcdef0123456789abcdef".into(),
            artifact: "ent_0123456789abcdef0123456789abcdef".into(),
            offset: 20,
            limit: 20,
        }),
    };
    let encoded = serde_json::to_vec(&request).expect("encode member request");
    let decoded: RequestEnvelope = serde_json::from_slice(&encoded).expect("decode member request");
    assert_eq!(decoded, request);

    let response = ResponseEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "artifact-members-1".into(),
        response: Response::ArtifactMembers(ArtifactMembersResponse {
            project: "prj_0123456789abcdef0123456789abcdef".into(),
            artifact: "ent_0123456789abcdef0123456789abcdef".into(),
            members: vec![ArtifactMemberSummary {
                key: "container:xl/workbook.xml".into(),
                project_path: None,
                container_part: Some("xl/workbook.xml".into()),
                digest: "ab".repeat(32),
            }],
            total: 41,
            next_offset: Some(21),
        }),
    };
    let encoded = serde_json::to_vec(&response).expect("encode member response");
    let decoded: ResponseEnvelope =
        serde_json::from_slice(&encoded).expect("decode member response");
    assert_eq!(decoded, response);
}

/// Defaults newer synchronization fields conservatively for older servers.
#[test]
fn artifact_summary_without_sync_fields_is_unavailable() {
    let value = serde_json::json!({
        "id": "artifact-1",
        "logical_key": "roads",
        "kind": "shapefile",
        "adapter": "shapefile",
        "adapter_version": null,
        "scan_id": "scan-1",
        "fidelity": "read-only-derived",
        "opaque_preservation": "unproven",
        "diagnostic": null,
        "references": [],
        "members": [],
        "member_count": 0
    });
    let summary: ArtifactSummary = serde_json::from_value(value).expect("legacy summary");
    assert_eq!(summary.baseline_digest, "");
    assert_eq!(summary.observed_digest, None);
    assert_eq!(summary.synchronization_status, "unavailable");
}
