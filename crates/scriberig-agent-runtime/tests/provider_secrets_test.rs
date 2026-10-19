//! Verifies real HTTP credential isolation, streaming redaction, and redirect denial.

use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider;
use scriberig_agent_runtime::provider::{
    CredentialSource, ModelMessage, ModelRequest, ProviderAdapter, ProviderConfig,
};
use scriberig_agent_runtime::{AgentMessage, ModelProvider};
use scriberig_secrets::{CredentialProvider, SecretError, SecretValue};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex, MutexGuard, OnceLock,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

static HTTP_FIXTURE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Supplies a different protected fake credential for each outbound request.
struct RotatingCredential {
    /// Number of per-request credential resolutions.
    calls: AtomicUsize,
}

impl CredentialProvider for RotatingCredential {
    /// Resolves the next protected fake credential.
    ///
    /// - `endpoint`: Actual destination supplied by the adapter.
    ///
    /// A rotating fake credential.
    ///
    /// Returns invalid-secret errors for malformed fixture values.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn credential(&self, endpoint: &str) -> Result<Option<SecretValue>, SecretError> {
        scriberig_secrets::normalize_endpoint(endpoint)?;
        let index = self.calls.fetch_add(1, Ordering::Relaxed);
        let credential = match index {
            0 => "CANARY_A",
            1 => "CANARY_B",
            _ => "CANARY_C",
        };
        SecretValue::new(credential.to_owned()).map(Some)
    }
}

/// Reads one bounded fixture request through its complete HTTP body.
///
/// - `stream`: Local fixture connection.
///
/// Captured request header and body containing only fake credential material.
/// # Returns
/// The operation result.
fn read_request(stream: &TcpStream) -> String {
    stream
        .set_nonblocking(false)
        .expect("blocking fixture connection");
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .expect("read timeout");
    let mut reader = BufReader::new(stream);
    let mut headers = String::new();
    let mut length = 0;
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).expect("HTTP header") > 0);
        if line.to_ascii_lowercase().starts_with("content-length:") {
            length = line
                .split(':')
                .nth(1)
                .expect("length")
                .trim()
                .parse::<usize>()
                .expect("numeric length");
        }
        headers.push_str(&line);
        if line == "\r\n" {
            break;
        }
        assert!(headers.len() < 32_768);
    }
    assert!(length < 65_536);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).expect("HTTP body");
    headers.push_str(std::str::from_utf8(&body).expect("JSON request"));
    headers
}

/// Identifies a nonblocking socket with no queued connection.
///
/// Windows reports `WSAEWOULDBLOCK` using a platform-specific error kind.
///
/// - `error`: Error returned by the nonblocking socket operation.
///
/// Whether the accept operation should be retried.
/// # Returns
/// `true` when the socket operation should be retried.
fn is_would_block(error: &std::io::Error) -> bool {
    let windows_would_block = {
        #[cfg(windows)]
        {
            error.raw_os_error() == Some(10035)
        }
        #[cfg(not(windows))]
        {
            false
        }
    };
    error.kind() == std::io::ErrorKind::WouldBlock || windows_would_block
}

/// Serializes tests that open local HTTP fixtures on Windows.
///
/// Windows endpoint inspection may transiently prevent concurrent fixtures
/// from receiving a loopback connection.
///
/// The fixture lock guard.
/// # Returns
/// A guard held for one local HTTP fixture test.
fn lock_http_fixture() -> MutexGuard<'static, ()> {
    HTTP_FIXTURE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Treats the Windows no-queued-connection error as retryable.
#[cfg(windows)]
#[test]
fn windows_would_block_socket_error_is_retryable() {
    let error = std::io::Error::from_raw_os_error(10035);
    assert!(is_would_block(&error));
}

/// Runs a bounded local HTTP fixture with one response per request.
///
/// - `listener`: Local listener bound before the provider starts.
/// - `responses`: Complete HTTP fixture responses.
///
/// A worker returning captured requests.
/// # Returns
/// The operation result.
fn serve(listener: TcpListener, responses: Vec<String>) -> std::thread::JoinHandle<Vec<String>> {
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    std::thread::spawn(move || {
        let mut requests = Vec::new();
        for response in responses {
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if is_would_block(&error) && Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            };
            requests.push(read_request(&stream));
            stream
                .write_all(response.as_bytes())
                .expect("fixture response");
        }
        requests
    })
}

/// Produces a complete successful fixture response with a bounded request ID.
///
/// - `body`: JSON or SSE response body.
/// - `request_id`: Fake request identifier.
///
/// A complete HTTP response.
/// # Returns
/// The operation result.
fn response(body: &str, request_id: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nx-request-id: {request_id}\r\n\r\n{body}",
        body.len()
    )
}

/// Verifies rotation and isolation on both real HTTP paths, including split reflections.
#[test]
fn credentials_are_headers_only_and_reflections_are_scrubbed() {
    let _fixture_lock = lock_http_fixture();
    let listener = TcpListener::bind("127.0.0.1:0").expect("local listener");
    let endpoint = format!("http://{}/v1", listener.local_addr().expect("address"));
    let normal =
        r#"{"choices":[{"message":{"content":"CANARY_A answer"},"finish_reason":"stop"}]}"#;
    let stream = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"CAN\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"ARY_B answer\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let reflected_stream = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"CAN\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"ARY_C answer\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let credentials = Arc::new(RotatingCredential {
        calls: AtomicUsize::new(0),
    });
    let provider = OpenAiCompatibleProvider::new(ProviderConfig::new(
        "test",
        endpoint,
        "model",
        CredentialSource::Authorized(credentials.clone()),
    ))
    .expect("provider");
    let check = r#"{"choices":[{"message":{"content":"MODEL OK"},"finish_reason":"stop"}]}"#;
    let worker = serve(
        listener,
        vec![
            response(normal, "CANARY_A"),
            response(stream, "trace-2"),
            response(check, "trace-3"),
            response(reflected_stream, "trace-4"),
        ],
    );
    let request = ModelRequest::new("model", vec![ModelMessage::user("hello")]);
    let normal_result = provider.complete(&request).expect("buffered result");
    let mut deltas = String::new();
    let streamed = provider
        .complete_streaming_with(&request, &mut |text| deltas.push_str(text))
        .expect("stream result");
    provider
        .check_connection("model")
        .expect("authenticated connection check");
    let mut model = ProviderModel::new(provider.clone(), "model");
    let decision = model
        .decide(&[AgentMessage::User("hello".to_owned())])
        .expect("provider model decision");
    let projections = model.take_request_projections();
    assert_eq!(credentials.calls.load(Ordering::Relaxed), 4);
    assert_eq!(normal_result.text.as_deref(), Some("[REDACTED] answer"));
    assert_eq!(deltas, "[REDACTED] answer");
    assert!(!format!("{normal_result:?}{streamed:?}").contains("CANARY"));
    assert_eq!(
        decision,
        scriberig_agent_runtime::ModelDecision::Final("[REDACTED] answer".to_owned())
    );
    assert_eq!(projections.len(), 1);
    assert!(!format!("{:?}", projections[0]).contains("CANARY_C"));
    let requests = worker.join().expect("fixture worker");
    for (request, credential) in requests
        .iter()
        .zip(["CANARY_A", "CANARY_B", "CANARY_C", "CANARY_C"])
    {
        assert!(
            request
                .to_ascii_lowercase()
                .contains(&format!("authorization: bearer {credential}").to_ascii_lowercase())
        );
        assert!(
            !request
                .split_once("\r\n\r\n")
                .expect("header/body")
                .1
                .contains(credential)
        );
    }
}

/// Redirects never forward a credential or trigger a second destination connection.
#[test]
fn both_http_paths_refuse_redirects() {
    let _fixture_lock = lock_http_fixture();
    let target = TcpListener::bind("127.0.0.1:0").expect("redirect target");
    target.set_nonblocking(true).expect("nonblocking target");
    let listener = TcpListener::bind("127.0.0.1:0").expect("source listener");
    let endpoint = format!("http://{}/v1", listener.local_addr().expect("address"));
    let redirect = format!(
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{}/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        target.local_addr().expect("target address")
    );
    let credentials = Arc::new(RotatingCredential {
        calls: AtomicUsize::new(0),
    });
    let provider = OpenAiCompatibleProvider::new(ProviderConfig::new(
        "test",
        endpoint,
        "model",
        CredentialSource::Authorized(credentials),
    ))
    .expect("provider");
    let worker = serve(listener, vec![redirect.clone(), redirect]);
    let request = ModelRequest::new("model", vec![ModelMessage::user("hello")]);
    assert!(provider.complete(&request).is_err());
    assert!(
        provider
            .complete_streaming_with(&request, &mut |_| {})
            .is_err()
    );
    worker.join().expect("source fixture");
    assert!(is_would_block(
        &target
            .accept()
            .expect_err("redirect destination never contacted")
    ));
}
