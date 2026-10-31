//! Verifies schema-redacted activity matches the cross-surface safe fixture.

use std::sync::Arc;

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService,
};
use scriberig_core::EntityId;
use scriberig_protocol::{
    ProtocolVersion, Request, RequestEnvelope, Response, SessionReadRequest, SessionReference,
};
use scriberig_server::Dispatcher;
use scriberig_session::SessionStore;
use scriberig_session::{
    Branch, CacheDisposition, ExecutionActivity, ModelInvocation, Session, ToolInvocation,
    ToolResult,
};
use scriberig_session_sqlite::SqliteSessionStore;

const SECRET_MARKER: &str = "FAKE_PROVIDER_SECRET_73B2";

/// Rejects unrelated app operations while providing durable session reads.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects the universal operation used by this session-only scenario.
    ///
    /// - `_context`: Trusted application context.
    /// - `_request`: Unrelated universal request.
    ///
    /// Never returns a successful universal operation.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "sensitive activity fixture".to_owned(),
        })
    }
}

/// Verifies the shared protocol activity omits the raw schema-sensitive value.
#[test]
fn protocol_activity_matches_the_safe_cross_surface_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/agent_console_sensitive_activity.json"
    ))
    .expect("cross-surface fixture");
    let secret = SECRET_MARKER;
    let session_id = EntityId::from_bytes([0x51; 16]);
    let branch_id = EntityId::from_bytes([0x52; 16]);
    let model_id = EntityId::from_bytes([0x53; 16]);
    let tool_id = EntityId::from_bytes([0x54; 16]);
    let result_id = EntityId::from_bytes([0x55; 16]);
    let mut branch = Branch::new(branch_id, "root").expect("branch");
    branch.model_invocations.push(ModelInvocation {
        id: model_id,
        branch: branch_id,
        operation: None,
        ordinal: 0,
        agent_profile: None,
        context_fingerprint: None,
        provider_profile: Some("local".to_owned()),
        provider_id: None,
        model: Some("model-a".to_owned()),
        model_profile: None,
        provider_model_id: None,
        thinking_level: Some("careful".to_owned()),
        request_fingerprint: None,
        cache: CacheDisposition::NotUsed,
        output: None,
        request: None,
    });
    branch.tool_invocations.push(ToolInvocation {
        id: tool_id,
        name: "search".to_owned(),
        input: serde_json::to_vec(&serde_json::json!({
            "query": "find documents",
            "token": secret,
        }))
        .expect("raw tool input"),
        parent_model_invocation: Some(model_id),
        presentation_input: Some(br#"{"query":"find documents","token":"[REDACTED]"}"#.to_vec()),
        presentation_label: Some("Find documents".to_owned()),
        category: Some(scriberig_session::ToolActivityKind::SemanticRead),
    });
    branch.tool_results.push(ToolResult {
        id: result_id,
        invocation: tool_id,
        output: serde_json::to_vec(&serde_json::json!({
            "summary": "Found one result",
            "private_payload": secret,
        }))
        .expect("raw tool result"),
        failed: false,
        presentation_preview: None,
    });
    branch.activity.extend([
        ExecutionActivity::ModelInvocation(model_id),
        ExecutionActivity::ToolInvocation(tool_id),
        ExecutionActivity::ToolResult(result_id),
    ]);
    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("session store"));
    store
        .save(&Session {
            id: session_id,
            branches: vec![branch],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("store session fixture");
    let service = ApplicationService::new(Fixture).with_session_store(store);
    let dispatcher = Dispatcher::new(&service);
    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "safe-activity".to_owned(),
        request: Request::SessionRead(SessionReadRequest {
            session: SessionReference {
                id: session_id.to_string(),
            },
            branch: None,
        }),
    });
    let Response::Session(response) = response.response else {
        panic!("unexpected session response: {:?}", response.response);
    };
    let mut actual =
        serde_json::to_value(&response.branches[0].activity).expect("serialize public activity");
    for item in actual.as_array_mut().expect("activity array") {
        item.as_object_mut()
            .expect("activity item")
            .retain(|_, value| !value.is_null());
    }

    assert_eq!(actual, fixture["protocolActivity"]);
    assert!(!actual.to_string().contains(secret));
    assert_eq!(
        response.branches[0].activity[1]
            .tool_arguments
            .as_ref()
            .unwrap()["token"],
        "[REDACTED]"
    );
}
