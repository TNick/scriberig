//! Verifies projection style contracts retain their typed JSON wire shape.

use scriberig_protocol::{
    ProjectionStylePurpose, ProjectionStyleResolveRequest, ProjectionStyleScope,
    ProjectionStyleSettingRequest, ProtocolVersion, Request, RequestEnvelope,
};

/// Verifies all style request variants serialize and deserialize without losing scope data.
#[test]
fn projection_style_requests_round_trip_as_protocol_envelopes() {
    let requests = [
        Request::ProjectionStyleResolve(ProjectionStyleResolveRequest {
            projection_kind: "javascript".to_owned(),
            project_id: "project".to_owned(),
            workspace_id: "workspace".to_owned(),
            purpose: ProjectionStylePurpose::InteractiveView,
            view_id: Some("view".to_owned()),
            explicit_personal: None,
        }),
        Request::ProjectionStyleSetting(ProjectionStyleSettingRequest {
            scope: ProjectionStyleScope::Project {
                project_id: "project".to_owned(),
            },
            projection_kind: "javascript".to_owned(),
            expected_revision: Some(9),
            style: Some(b"quote=single".to_vec()),
        }),
    ];

    for (index, request) in requests.into_iter().enumerate() {
        let envelope = RequestEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: format!("style-{index}"),
            request,
        };
        let value = serde_json::to_value(&envelope).expect("request serializes");
        let decoded: RequestEnvelope = serde_json::from_value(value).expect("request decodes");
        assert_eq!(decoded, envelope);
    }
}
