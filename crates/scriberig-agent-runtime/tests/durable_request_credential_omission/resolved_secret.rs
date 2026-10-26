//! Verifies resolved provider secrets stay outside durable `ScribeRig` state.

use std::io::{Read, Write};
use std::thread::JoinHandle;

use super::*;
use scriberig_secrets::{CredentialProvider, SecretError, SecretValue};

/// Resolves one fixture credential for the exact provider endpoint.
struct FixtureCredentials;

impl CredentialProvider for FixtureCredentials {
    /// Returns the request-time sentinel as protected material.
    ///
    /// - `_endpoint`: Provider endpoint selected by the invocation.
    ///
    /// The host-protected credential.
    /// # Returns
    /// The credential used by the local HTTP fixture.
    /// # Errors
    /// Returns an error only if the test sentinel cannot be wrapped.
    fn credential(&self, _endpoint: &str) -> Result<Option<SecretValue>, SecretError> {
        SecretValue::new(CREDENTIAL_SENTINEL.to_owned()).map(Some)
    }
}

/// Starts a local Responses endpoint and captures its request headers/body.
///
/// # Returns
/// The model endpoint URL and a join handle yielding the captured request.
/// # Panics
/// Panics when the test listener cannot bind or the HTTP exchange is malformed.
fn responses_fixture() -> (String, JoinHandle<(String, String)>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind local provider");
    let address = listener.local_addr().expect("provider address");
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept provider request");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        let (header_end, content_length) = loop {
            let read = socket.read(&mut chunk).expect("read provider request");
            assert!(read > 0, "provider closed before sending request headers");
            request.extend_from_slice(&chunk[..read]);
            if let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().expect("content length"))
                    })
                    .expect("request content length");
                break (header_end, content_length);
            }
        };
        let body_start = header_end + 4;
        while request.len() < body_start + content_length {
            let read = socket.read(&mut chunk).expect("read provider body");
            assert!(read > 0, "provider closed before sending the full body");
            request.extend_from_slice(&chunk[..read]);
        }
        let headers = String::from_utf8_lossy(&request[..header_end]).into_owned();
        let body =
            String::from_utf8_lossy(&request[body_start..body_start + content_length]).into_owned();
        let response_body = concat!(
            "event: response.completed\r\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_safe\",\"status\":\"completed\",\"output\":[]}}\r\n",
            "\r\n",
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        socket
            .write_all(response.as_bytes())
            .expect("write provider response");
        (headers, body)
    });
    (format!("http://{address}/v1"), server)
}

/// Persists captured request provenance and returns its authorized detail.
///
/// - `model`: Exact upstream model used by the invocation.
/// - `catalog`: Shared catalog containing the selected model.
/// - `captured`: Safe request projections produced by the invocation.
///
/// The historical request detail reloaded from a fresh SQLite handle.
/// # Returns
/// The authorized stored model-request projection.
/// # Panics
/// Panics when session creation, persistence, authorization, or reload fails.
fn persist_and_reload_detail(
    model: &rpi_ai::Model,
    catalog: rpi_model_runtime::ModelCatalog,
    captured: &[ModelRequestProjection],
) -> ModelRequestProjection {
    let directory = tempfile::tempdir().expect("temporary session directory");
    let path = directory
        .path()
        .join("scriberig-credential-omission.sqlite");
    let session = EntityId::from_bytes([62; 16]);
    let store: Arc<dyn scriberig_session::SessionStore> =
        Arc::new(SqliteSessionStore::open(&path).expect("open durable session store"));
    let writer = ApplicationService::new(Fixture)
        .with_session_store(Arc::clone(&store))
        .with_model_catalog(catalog.clone());
    let handle = writer
        .start_local_agent(&AgentCommandRequest {
            session,
            branch: None,
            anchor: None,
            prompt: "review the bounded request".to_owned(),
            selection: None,
        })
        .expect("start local agent");
    writer
        .complete_local_agent_with_evidence(
            &handle,
            "safe completed answer",
            Vec::new(),
            Vec::new(),
            captured.to_vec(),
        )
        .expect("persist the captured request");

    let context = authorized_context();
    let reader = ApplicationService::new(Fixture)
        .with_session_store(Arc::new(
            SqliteSessionStore::open(&path).expect("reopen durable session store"),
        ))
        .with_model_catalog(catalog);
    let durable = SessionBackend::read_session(&reader, &context, &SessionRequest { session })
        .expect("reload durable session graph");
    assert!(
        !serialized_contains_credential(&durable, "reloaded durable session"),
        "the SQLite reload leaked a provider credential"
    );
    let detail = AgentConsoleBackend::model_request_detail(
        &reader,
        &context,
        session,
        handle.root_branch,
        durable.branches[0].model_invocations[0].id,
    )
    .expect("authorized historical request detail");
    assert!(
        !serialized_contains_credential(&detail, "authorized request detail"),
        "the authorized detail leaked a provider credential"
    );
    assert!(
        !serialized_contains_credential(
            &durable.branches[0].model_invocations,
            "durable model invocation"
        ),
        "the durable invocation leaked a provider credential"
    );
    drop(reader);
    drop(writer);
    drop(store);
    let stored_bytes = std::fs::read(&path).expect("read the durable SQLite file");
    assert!(
        !String::from_utf8_lossy(&stored_bytes).contains(CREDENTIAL_SENTINEL),
        "the durable SQLite payload leaked the provider credential"
    );
    assert_eq!(detail.model, model.id);
    detail
}

/// Keeps a resolved request-only secret out of capture, persistence, and detail.
#[test]
fn resolved_request_secret_is_transmitted_only_to_the_provider() {
    let (endpoint, server) = responses_fixture();
    let mut model = rpi_ai::providers::openai_responses::openai_responses_models().remove(0);
    model.base_url = endpoint;
    let provider = Arc::new(
        rpi_ai::providers::openai_responses::OpenAiResponsesProvider::with_models_without_env_api_key(
            "openai",
            None,
            reqwest::Client::new(),
            vec![model.clone()],
        ),
    );
    let credential = FixtureCredentials;
    let request_secret = scriberig_agent_runtime::protected_credential::request_secret_for_model(
        &credential,
        &model,
    )
    .expect("resolve exact destination credential")
    .expect("fixture credential");

    let captured = capture_requests(provider, &model, Some(request_secret), true);
    let (headers, body) = server.join().expect("local provider request");
    assert!(
        headers.contains(&format!("Bearer {CREDENTIAL_SENTINEL}")),
        "upstream provider must receive the resolved secret as bearer authorization"
    );
    assert!(
        !body.contains(CREDENTIAL_SENTINEL),
        "request body must not contain the resolved credential"
    );
    assert_eq!(captured.len(), 1);
    let applied_settings = captured[0]
        .applied_settings
        .as_ref()
        .expect("built-in provider reports applied request settings");
    assert_eq!(
        applied_settings["schemaVersion"],
        serde_json::json!("rpi-applied-request-settings-v5")
    );
    assert_eq!(applied_settings["complete"], serde_json::json!(true));
    assert_eq!(
        applied_settings["requestEffectSha256"]
            .as_str()
            .expect("protected request effect digest")
            .len(),
        64
    );
    assert!(
        !serialized_contains_credential(&captured, "resolved-secret request projection"),
        "request provenance leaked the resolved credential"
    );

    let catalog = rpi_model_runtime::ModelCatalog::new();
    catalog.reconcile_models(vec![model.clone()]);
    let detail = persist_and_reload_detail(&model, catalog, &captured);
    assert_eq!(detail, captured[0]);
    assert_eq!(detail.provider_id.as_deref(), Some("openai"));
    assert!(
        !serialized_contains_credential(&detail, "resolved-secret protocol detail"),
        "protocol detail leaked the resolved credential"
    );
}
