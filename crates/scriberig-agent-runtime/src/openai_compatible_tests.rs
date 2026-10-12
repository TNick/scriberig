//! Tests the OpenAI-compatible wire conversion and response normalization.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::openai_transport::{CompletionTransport, TransportResponse};
use super::{
    OpenAiCompatibleProvider, WireRequest, WireResponse, openai_transport::classify_status,
};
use crate::provider::{
    CredentialSource, FinishReason, ModelMessage, ModelRequest, ProviderAdapter, ProviderConfig,
    ProviderError, ToolDefinition,
};

/// Records one request received by the in-process provider fixture.
#[derive(Clone, Debug, Eq, PartialEq)]
struct MockRequest {
    /// Complete endpoint selected by the provider.
    endpoint: String,
    /// JSON request payload sent to the fixture.
    payload: serde_json::Value,
    /// Optional bearer credential sent to the fixture.
    credential: Option<String>,
    /// Configured request deadline.
    timeout: Duration,
}

/// Supplies deterministic responses without opening a network socket.
#[derive(Clone)]
struct MockCompletionTransport {
    /// Response status returned to the provider.
    status: reqwest::StatusCode,
    /// Response body returned to the provider.
    body: Vec<u8>,
    /// Optional safe request identifier.
    request_id: Option<String>,
    /// Requests recorded for assertions.
    requests: Arc<Mutex<Vec<MockRequest>>>,
}

impl CompletionTransport for MockCompletionTransport {
    /// Records one request and returns the configured response.
    ///
    /// # Parameters
    /// - `endpoint`: Complete provider endpoint.
    /// - `payload`: Provider request payload.
    /// - `credential`: Optional bearer credential.
    /// - `timeout`: Configured request deadline.
    ///
    /// # Returns
    /// The configured mock response.
    ///
    /// # Errors
    /// Returns a transport error when request recording is poisoned.
    fn send(
        &self,
        endpoint: &str,
        payload: &WireRequest,
        credential: Option<&str>,
        timeout: Duration,
    ) -> Result<TransportResponse, ProviderError> {
        let payload = serde_json::to_value(payload)
            .map_err(|error| ProviderError::Transport(error.to_string()))?;
        self.requests
            .lock()
            .map_err(|_| ProviderError::Transport("mock request lock failed".to_owned()))?
            .push(MockRequest {
                endpoint: endpoint.to_owned(),
                payload,
                credential: credential.map(str::to_owned),
                timeout,
            });
        Ok(TransportResponse {
            status: self.status,
            request_id: self.request_id.clone(),
            body: self.body.clone(),
        })
    }
}

/// Holds a provider request until cancellation wins the adapter race.
struct BlockingCompletionTransport {
    /// Release flag used to let the worker exit after the assertion.
    release: Arc<AtomicBool>,
}

impl CompletionTransport for BlockingCompletionTransport {
    /// Waits for release, then returns a deterministic transport failure.
    ///
    /// # Parameters
    /// - `_endpoint`: Complete provider endpoint.
    /// - `_payload`: Provider request payload.
    /// - `_credential`: Optional bearer credential.
    /// - `_timeout`: Configured request deadline.
    ///
    /// # Returns
    /// Never returns successfully.
    ///
    /// # Errors
    /// Returns a deterministic transport failure after release.
    fn send(
        &self,
        _endpoint: &str,
        _payload: &WireRequest,
        _credential: Option<&str>,
        _timeout: Duration,
    ) -> Result<TransportResponse, ProviderError> {
        while !self.release.load(std::sync::atomic::Ordering::Acquire) {
            thread::sleep(Duration::from_millis(1));
        }
        thread::sleep(Duration::from_millis(100));
        Err(ProviderError::Transport("released mock request".to_owned()))
    }
}

/// Verifies provider-neutral tool definitions serialize to function tools.
#[test]
fn request_serialization_contains_model_messages_and_tools() {
    let mut request = ModelRequest::new("model", vec![ModelMessage::user("hello")]);
    request.tools.push(ToolDefinition {
        name: "inspect".to_owned(),
        description: "Inspect an entity".to_owned(),
        parameters: serde_json::json!({"type": "object"}),
    });

    let wire = serde_json::to_value(WireRequest::from_request(&request, false)).expect("wire JSON");

    assert_eq!(wire["model"], "model");
    assert_eq!(wire["messages"][0]["role"], "user");
    assert_eq!(wire["tools"][0]["function"]["name"], "inspect");
    let streaming = serde_json::to_value(WireRequest::from_request(&request, true))
        .expect("streaming wire JSON");
    assert_eq!(streaming["stream"], true);
}

/// Verifies fragmented provider function arguments are parsed only when complete.
#[test]
fn response_normalization_parses_structured_tool_arguments() {
    let wire: WireResponse = serde_json::from_value(serde_json::json!({
        "model": "model",
        "choices": [{
            "message": {
                "content": null,
                "tool_calls": [{
                    "id": "call-1",
                    "function": {
                        "name": "inspect",
                        "arguments": r#"{"entity":"ent_1"}"#
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    }))
    .expect("wire response");

    let response = wire
        .into_response(Some("request-1".to_owned()))
        .expect("response");

    assert_eq!(response.finish_reason, FinishReason::ToolCall);
    assert_eq!(response.tool_calls[0].arguments["entity"], "ent_1");
    assert_eq!(response.metadata.request_id.as_deref(), Some("request-1"));
}

/// Verifies cancellation is rejected before an outbound request starts.
#[test]
fn cancellation_is_observed_before_provider_request() {
    let provider = OpenAiCompatibleProvider::new(ProviderConfig::new(
        "mock",
        "http://127.0.0.1:1",
        "mock-model",
        CredentialSource::None,
    ))
    .expect("provider");
    let cancelled = AtomicBool::new(true);

    let error = provider
        .complete_with_cancellation(
            &ModelRequest::new("mock-model", vec![ModelMessage::user("hello")]),
            &cancelled,
        )
        .expect_err("cancelled request");

    assert_eq!(error, ProviderError::Cancelled);
}

/// Verifies provider deadline statuses normalize to timeout failures.
#[test]
fn request_timeout_status_is_classified() {
    let error = classify_status(reqwest::StatusCode::REQUEST_TIMEOUT, None);
    assert!(matches!(error, ProviderError::Timeout(_)));
}

/// Verifies the concrete adapter normalizes a local mock text response.
#[test]
fn text_response_round_trips_through_local_mock_transport() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = MockCompletionTransport {
        status: reqwest::StatusCode::OK,
        body: br#"{"model":"mock-model","choices":[{"message":{"content":"hello from mock"},"finish_reason":"stop"}]}"#.to_vec(),
        request_id: Some("mock-request-1".to_owned()),
        requests: Arc::clone(&requests),
    };
    let provider = OpenAiCompatibleProvider::with_transport(
        ProviderConfig::new(
            "mock",
            "http://mock.local/v1",
            "mock-model",
            CredentialSource::None,
        ),
        Arc::new(transport),
    )
    .expect("provider");

    let response = provider
        .complete(&ModelRequest::new(
            "mock-model",
            vec![ModelMessage::user("hello")],
        ))
        .expect("mock completion");

    assert_eq!(response.text.as_deref(), Some("hello from mock"));
    assert_eq!(
        response.metadata.request_id.as_deref(),
        Some("mock-request-1")
    );
    let requests = requests.lock().expect("mock requests");
    assert_eq!(
        requests[0].endpoint,
        "http://mock.local/v1/chat/completions"
    );
    assert_eq!(requests[0].payload["model"], "mock-model");
    assert_eq!(requests[0].payload["messages"][0]["content"], "hello");
    assert_eq!(requests[0].credential, None);
}

/// Verifies configured timeout status and active cancellation remain structured.
#[test]
fn timeout_and_cancellation_terminate_with_structured_errors() {
    let timeout_provider = OpenAiCompatibleProvider::with_transport(
        ProviderConfig::new(
            "mock",
            "http://mock.local",
            "mock-model",
            CredentialSource::None,
        ),
        Arc::new(MockCompletionTransport {
            status: reqwest::StatusCode::REQUEST_TIMEOUT,
            body: Vec::new(),
            request_id: None,
            requests: Arc::new(Mutex::new(Vec::new())),
        }),
    )
    .expect("timeout provider");
    let request = ModelRequest::new("mock-model", vec![ModelMessage::user("hello")]);
    assert!(matches!(
        timeout_provider.complete(&request),
        Err(ProviderError::Timeout(_))
    ));

    let release = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancellation_provider = OpenAiCompatibleProvider::with_transport(
        ProviderConfig::new(
            "mock",
            "http://mock.local",
            "mock-model",
            CredentialSource::None,
        ),
        Arc::new(BlockingCompletionTransport {
            release: Arc::clone(&release),
        }),
    )
    .expect("cancellation provider");
    let cancellation_signal = Arc::clone(&cancelled);
    let release_signal = Arc::clone(&release);
    let signal = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        cancellation_signal.store(true, std::sync::atomic::Ordering::Release);
        release_signal.store(true, std::sync::atomic::Ordering::Release);
    });
    let error = cancellation_provider
        .complete_with_cancellation(&request, &cancelled)
        .expect_err("cancellation");
    signal.join().expect("cancellation signal");
    assert_eq!(error, ProviderError::Cancelled);
}
