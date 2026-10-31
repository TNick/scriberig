//! Implements the deterministic and brokered provider callbacks used by the fixture.

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use rpi_plugin_sdk::{StbString, StbStringRef};

use super::{
    BROKER_ENDPOINT_ENV, BROKER_MODEL_ID, BROKER_PROVIDER_ID, MODEL_ID, PROVIDER_ID,
    SEMANTIC_REQUEST_PREFIX, TOOL_NAME,
};

static SEMANTIC_PROVIDER_REQUESTS: AtomicUsize = AtomicUsize::new(0);

/// Reads the first user text block from an upstream provider request.
///
/// - `request`: Serialized upstream provider request.
///
/// # Returns
/// The first user message text when it is available.
fn first_user_text(request: &serde_json::Value) -> Option<&str> {
    request
        .pointer("/context/messages")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .find(|message| message.get("role").and_then(serde_json::Value::as_str) == Some("user"))
        .and_then(|message| message.get("content"))
        .and_then(|content| {
            content.as_str().or_else(|| {
                content
                    .as_array()?
                    .iter()
                    .find_map(|block| block.get("text").and_then(serde_json::Value::as_str))
            })
        })
}

rpi_plugin_sdk::string_handler! {
    /// Returns a deterministic assistant message to the isolated host.
    ///
    /// # Parameters
    /// - `_request`: Provider request envelope used to detect the fixture tool result.
    /// - `output`: Output slot filled with the serialized assistant response.
    /// - `_user_data`: Opaque registration context, unused by the fixture.
    ///
    /// # Returns
    /// Zero when the response was written, or one when the callback panics.
    pub(super) fn provider_request(_request, output, _user_data) {
        let request: serde_json::Value = serde_json::from_str(_request).unwrap_or_default();
        let tool_result = request
            .pointer("/context/messages")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|messages| {
                messages.iter().any(|message| {
                    message.get("role").and_then(serde_json::Value::as_str) == Some("toolResult")
                        && message
                            .to_string()
                            .contains("isolated fixture tool completed")
                })
            });
        let tool_registration_count = request
            .pointer("/context/tools")
            .and_then(serde_json::Value::as_array)
            .map_or(0, |tools| {
                tools
                    .iter()
                    .filter(|tool| tool["name"] == TOOL_NAME)
                    .count()
            });
        let semantic_result = request
            .pointer("/context/messages")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|messages| {
                messages.iter().any(|message| {
                    message
                        .to_string()
                        .contains("candidate:plugin-domain-operation")
                })
            });
        let tool_arguments = first_user_text(&request)
            .and_then(|text| text.strip_prefix(SEMANTIC_REQUEST_PREFIX))
            .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let semantic_request = first_user_text(&request)
            .is_some_and(|text| text.starts_with(SEMANTIC_REQUEST_PREFIX));
        let repeated_semantic_request = semantic_request
            && SEMANTIC_PROVIDER_REQUESTS.fetch_add(1, Ordering::Relaxed) > 0;
        let (content, stop_reason) = if tool_registration_count > 1 {
            (
                serde_json::json!([{
                    "type": "text",
                    "text": "isolated fixture provider saw duplicate tool registration"
                }]),
                "stop",
            )
        } else if repeated_semantic_request || (tool_result && semantic_result) {
            (
                serde_json::json!([{
                    "type": "text",
                    "text": "fixture semantic operation completed"
                }]),
                "stop",
            )
        } else if tool_result {
            (
                serde_json::json!([{
                    "type": "text",
                    "text": "isolated fixture provider observed tool result"
                }]),
                "stop",
            )
        } else if tool_registration_count == 1 {
            (
                serde_json::json!([{
                    "type": "toolCall",
                    "id": "fixture-tool-call",
                    "name": TOOL_NAME,
                    "arguments": tool_arguments
                }]),
                "tooluse",
            )
        } else {
            (
                serde_json::json!([{
                    "type": "text",
                    "text": "isolated fixture provider completed"
                }]),
                "stop",
            )
        };
        let response = serde_json::json!({
            "role": "assistant",
            "content": content,
            "api": "faux",
            "provider": PROVIDER_ID,
            "model": MODEL_ID,
            "usage": {
                "input": 1,
                "output": 1,
                "cacheRead": 0,
                "cacheWrite": 0,
                "totalTokens": 2,
                "cost": {"input": 0.0, "output": 0.0, "cacheRead": 0.0, "cacheWrite": 0.0, "total": 0.0}
            },
            "stopReason": stop_reason,
            "timestamp": 1
        });
        i32::from(!output.write(response.to_string()))
    }
}

/// Sends one provider request through the protected host HTTP callback.
///
/// # Parameters
/// - `request_json`: Upstream provider invocation envelope.
/// - `control`: Host-provided protected provider callbacks.
/// - `output`: Plugin-owned assistant response slot.
/// - `_user_data`: Opaque registration context, unused by the fixture.
///
/// # Returns
/// Zero when the host response is converted into an assistant message.
pub(super) extern "C" fn brokered_provider_request(
    request_json: StbStringRef,
    control: *const rpi_plugin_sdk::ProtectedProviderControl,
    output: *mut StbString,
    _user_data: *mut c_void,
) -> i32 {
    if output.is_null() || control.is_null() {
        return 1;
    }
    // SAFETY: the host owns and lends the protected control for this callback.
    let control = unsafe { &*control };
    let required_size =
        std::mem::offset_of!(rpi_plugin_sdk::ProtectedProviderControl, request_http)
            + std::mem::size_of::<Option<rpi_plugin_sdk::ProviderHttpRequestFn>>();
    if (control.struct_size as usize) < required_size {
        return 1;
    }
    let Some(request_http) = control.request_http else {
        return 1;
    };
    // SAFETY: the host lends this UTF-8 request for the duration of the callback.
    let request_json = unsafe { request_json.as_str() };
    let request: serde_json::Value = serde_json::from_str(request_json).unwrap_or_default();
    let endpoint = std::env::var(BROKER_ENDPOINT_ENV).unwrap_or_default();
    if endpoint.is_empty() {
        return 1;
    }
    let Some(address) = endpoint
        .strip_prefix("http://")
        .and_then(|value| value.parse::<std::net::SocketAddr>().ok())
    else {
        return 1;
    };
    if std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_millis(200)).is_ok()
    {
        return 1;
    }
    let prepared = serde_json::json!({
        "schemaVersion": "rpi-provider-http-request-v1",
        "method": "POST",
        "url": format!("{endpoint}/chat/completions"),
        "headers": {"authorization": "Bearer fixture-credential-sentinel"},
        "body": {
            "model": BROKER_MODEL_ID,
            "prompt": first_user_text(&request).unwrap_or("fixture-prompt-sentinel")
        }
    });
    let prepared = prepared.to_string();
    let mut response_body = None::<String>;
    let result = request_http(
        StbStringRef::from_str(&prepared),
        consume_broker_response,
        (&raw mut response_body).cast::<c_void>(),
        control.user_data,
    );
    let Some(answer) = response_body.filter(|_| result == 0) else {
        return 1;
    };
    let response = serde_json::json!({
        "role": "assistant",
        "content": [{"type": "text", "text": answer}],
        "api": "openai-completions",
        "provider": BROKER_PROVIDER_ID,
        "model": BROKER_MODEL_ID,
        "usage": {
            "input": 1,
            "output": 1,
            "cacheRead": 0,
            "cacheWrite": 0,
            "totalTokens": 2,
            "cost": {"input": 0.0, "output": 0.0, "cacheRead": 0.0, "cacheWrite": 0.0, "total": 0.0}
        },
        "stopReason": "stop",
        "timestamp": 1
    });
    // SAFETY: the host provides a writable response slot for this callback.
    unsafe { *output = StbString::from_string(response.to_string()) };
    0
}

/// Capture the answer body returned by the host's provider HTTP broker.
///
/// # Parameters
/// - `response`: Host-owned JSON response valid during this callback.
/// - `user_data`: Live stack slot for the captured answer.
///
/// # Returns
/// Zero when the response contains an answer, otherwise one.
pub(super) extern "C" fn consume_broker_response(
    response: StbStringRef,
    user_data: *mut c_void,
) -> i32 {
    if user_data.is_null() {
        return 1;
    }
    // SAFETY: the callback is synchronous and receives a live local slot.
    let output = unsafe { &mut *user_data.cast::<Option<String>>() };
    // SAFETY: the host lends response bytes for this synchronous callback.
    let response = unsafe { response.as_str() };
    let value: serde_json::Value = serde_json::from_str(response).unwrap_or_default();
    *output = value
        .pointer("/body/answer")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    i32::from(output.is_none())
}
