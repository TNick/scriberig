//! Verifies `ScribeRig` delegates Responses continuation to the upstream provider.

use std::io::{Read, Write};
use std::sync::Arc;
use std::thread::JoinHandle;

use super::*;
use scriberig_agent_runtime::upstream_invocation::run_upstream_branch_invocation_with_continuation;

/// Builds one branch invocation for the upstream Responses provider.
///
/// - `provider`: Upstream provider instance serving the local HTTP fixture.
/// - `model`: Exact model used for both turns.
///
/// The `ScribeRig` branch invocation with no tools or persisted provider state.
/// # Returns
/// A fresh invocation bound to the exact provider and model.
fn invocation(
    provider: Arc<dyn rpi_ai::Provider>,
    model: &rpi_ai::Model,
) -> UpstreamBranchInvocation {
    UpstreamBranchInvocation {
        branch: EntityId::from_bytes([71; 16]),
        provider,
        model: model.clone(),
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: Some("loopback-openai-responses".to_owned()),
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context: ContextCompiler::new(ContextPolicy::default()).compile(
            &BranchExecutionContext::new(
                "continuation-session",
                "continuation-branch",
                "upstream continuation integration",
                "continue using the provider response",
            ),
        ),
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        provider_lifecycle: None,
    }
}

/// Starts a two-turn loopback Responses endpoint.
///
/// # Returns
/// The model endpoint URL and a join handle yielding both captured requests.
/// # Panics
/// Panics when the local listener cannot bind or the provider exchange is invalid.
fn continuation_responses_fixture() -> (String, JoinHandle<Vec<(String, String)>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind local provider");
    let address = listener.local_addr().expect("provider address");
    let server = std::thread::spawn(move || {
        let mut captured = Vec::new();
        for response_id in ["resp_first", "resp_second"] {
            let (mut socket, _) = listener.accept().expect("accept provider request");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 4096];
            let (header_end, content_length) = loop {
                let read = socket.read(&mut chunk).expect("read request headers");
                assert!(read > 0, "provider closed before sending request headers");
                request.extend_from_slice(&chunk[..read]);
                if let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n")
                {
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
                let read = socket.read(&mut chunk).expect("read request body");
                assert!(read > 0, "provider closed before sending the request body");
                request.extend_from_slice(&chunk[..read]);
            }
            let headers = String::from_utf8_lossy(&request[..header_end]).into_owned();
            let body = String::from_utf8_lossy(&request[body_start..body_start + content_length])
                .into_owned();
            captured.push((headers, body));

            let response_body = format!(
                "event: response.created\r\ndata: {{\"type\":\"response.created\",\"response\":{{\"id\":\"{response_id}\"}}}}\r\n\r\nevent: response.output_text.delta\r\ndata: {{\"type\":\"response.output_text.delta\",\"delta\":\"{response_id}\"}}\r\n\r\nevent: response.completed\r\ndata: {{\"type\":\"response.completed\",\"response\":{{\"id\":\"{response_id}\",\"status\":\"completed\",\"output\":[]}}}}\r\n\r\n"
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            socket
                .write_all(response.as_bytes())
                .expect("write provider response");
        }
        captured
    });
    (format!("http://{address}/v1"), server)
}

/// Continues a real upstream Responses interaction without replaying history.
#[test]
fn upstream_responses_continuation_is_extracted_and_sent_by_the_provider() {
    let (endpoint, server) = continuation_responses_fixture();
    let mut model = rpi_ai::providers::openai_responses::openai_responses_models().remove(0);
    model.base_url = endpoint;
    let provider = Arc::new(
        rpi_ai::providers::openai_responses::OpenAiResponsesProvider::with_models_without_env_api_key(
            "openai",
            Some("local-test-key".to_owned()),
            reqwest::Client::new(),
            vec![model.clone()],
        ),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("provider continuation runtime");

    let (first, second) = runtime.block_on(async {
        let (first_result, continuation) = run_upstream_branch_invocation_with_continuation(
            invocation(provider.clone(), &model),
            "first user turn",
            tokio_util::sync::CancellationToken::new(),
            None,
            None,
            None,
        )
        .await
        .expect("first upstream response");
        assert_eq!(first_result.output, "resp_first");
        let continuation = continuation.expect("provider response continuation");

        let (second_result, next_continuation) = run_upstream_branch_invocation_with_continuation(
            invocation(provider, &model),
            "second user turn",
            tokio_util::sync::CancellationToken::new(),
            None,
            None,
            Some(continuation),
        )
        .await
        .expect("continued upstream response");
        assert_eq!(second_result.output, "resp_second");
        assert!(next_continuation.is_some());
        (first_result, second_result)
    });

    assert_eq!(first.model_requests.len(), 1);
    assert_eq!(second.model_requests.len(), 1);
    let requests = server.join().expect("two provider requests");
    assert_eq!(requests.len(), 2);
    let second_body: serde_json::Value =
        serde_json::from_str(&requests[1].1).expect("second request body");
    assert_eq!(second_body["previous_response_id"], "resp_first");
    assert_eq!(second_body["input"][0]["content"], "second user turn");
    assert!(
        !requests[1].1.contains("first user turn"),
        "provider-owned continuation should not resend the earlier prompt"
    );
}
