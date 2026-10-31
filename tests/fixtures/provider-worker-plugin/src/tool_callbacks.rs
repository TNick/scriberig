//! Implements the native fixture tool and command callbacks.

use std::ffi::c_void;

use rpi_plugin_sdk::{
    RuntimeActionId, StbString, StbStringRef, StepHandle, StepResult, ToolPartialCb,
    callbacks::{OwnedString, free_string},
};

use super::RUNTIME_ACTION;

/// Returns a structured message for the isolated worker command fixture.
///
/// - `args_json`: Borrowed upstream command envelope.
/// - `output`: Plugin-owned structured response output.
/// - `_user_data`: Opaque registration data unused by this fixture.
///
/// # Returns
/// Zero after writing a JSON message response.
pub(super) extern "C" fn fixture_command(
    args_json: StbStringRef,
    output: *mut StbString,
    _user_data: *mut c_void,
) -> i32 {
    if output.is_null() {
        return 1;
    }
    // SAFETY: the host lends this UTF-8 buffer for the duration of the callback.
    let input = unsafe { args_json.as_str() }.to_owned();
    let arguments = serde_json::from_str::<serde_json::Value>(&input)
        .ok()
        .and_then(|value| {
            value
                .get("args")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    let action = serde_json::from_str::<serde_json::Value>(&input)
        .ok()
        .and_then(|value| value.get("action").cloned());
    let response = serde_json::json!({
        "kind": "message",
        "text": action.map_or_else(
            || format!("fixture command received: {arguments}"),
            |action| {
                let value = action
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                format!("fixture command action received: {value}")
            },
        ),
    })
    .to_string();
    // SAFETY: the ABI caller supplies a writable output slot.
    unsafe { *output = StbString::from_string(response) };
    0
}

/// Starts the deterministic fixture tool and releases its unused arguments.
///
/// # Parameters
/// - `_call_id`: Host-generated call identity, unused by this fixture.
/// - `params`: Owned serialized arguments supplied by the host.
/// - `free_params`: Host callback that releases `params`.
///
/// # Returns
/// An opaque handle for the completed fixture tool result.
pub(super) extern "C" fn fixture_tool_execute(
    _call_id: StbStringRef,
    params: StbString,
    free_params: Option<rpi_plugin_sdk::FreeStringFn>,
) -> StepHandle {
    // SAFETY: the worker transfers this owned string with its producer callback.
    let owned = unsafe { OwnedString::from_raw(params, free_params) };
    let mut arguments: serde_json::Value =
        serde_json::from_str(&owned.to_string_lossy()).unwrap_or_default();
    let mut results = Vec::new();
    if arguments.get("hostCapability").is_some() {
        results.push(invoke_host_capability(&mut arguments));
    }
    if arguments.get("hostComponent").is_some() {
        results.push(invoke_host_component(&arguments));
    }
    let result = if results.is_empty() {
        "isolated fixture tool completed".to_owned()
    } else {
        results.join("; ")
    };
    Box::into_raw(Box::new(result)) as StepHandle
}

/// Invokes one explicitly requested capability from an active tool call.
///
/// # Parameters
/// - `request`: Host capability name and arguments supplied by this fixture.
///
/// # Returns
/// A readable host result or denial.
fn invoke_host_capability(request: &mut serde_json::Value) -> String {
    let Some(action) = RUNTIME_ACTION.get() else {
        return "fixture host action is unavailable".to_owned();
    };
    let Some(host_context) = request.get("__rpi") else {
        return "fixture host context is unavailable".to_owned();
    };
    let Some(handle) = host_context
        .get("runtimeHandle")
        .and_then(serde_json::Value::as_u64)
    else {
        return "fixture runtime handle is unavailable".to_owned();
    };
    let Ok(handle) = usize::try_from(handle) else {
        return "fixture runtime handle exceeds this platform's pointer width".to_owned();
    };
    let Some(runtime_key) = host_context
        .get("runtimeKey")
        .and_then(serde_json::Value::as_str)
    else {
        return "fixture runtime key is unavailable".to_owned();
    };
    let mut response = StbString::empty();
    let capability = request
        .get("hostCapability")
        .unwrap_or(&serde_json::Value::Null);
    let encoded = serde_json::json!({
        "capability": capability.get("name").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "arguments": capability.get("arguments").cloned().unwrap_or_else(|| serde_json::json!({})),
        "__rpiRuntimeKey": runtime_key,
    })
    .to_string();
    let status = action(
        RuntimeActionId::HostCapability.into(),
        StbStringRef::from_str(&encoded),
        &raw mut response,
        handle as *mut c_void,
    );
    // SAFETY: the host writes a uniquely owned response through the output slot.
    let response = unsafe { OwnedString::from_raw(response, Some(free_string)) };
    if status == 0 {
        format!("host capability result: {}", response.to_string_lossy())
    } else {
        format!("host capability denied: {}", response.to_string_lossy())
    }
}

/// Publishes one component through the isolated worker's upstream runtime bridge.
///
/// # Parameters
/// - `request`: Tool arguments containing `hostComponent` and the runtime handle.
///
/// # Returns
/// A readable acknowledgement or denial.
fn invoke_host_component(request: &serde_json::Value) -> String {
    let Some(action) = RUNTIME_ACTION.get() else {
        return "fixture host action is unavailable".to_owned();
    };
    let Some(handle) = request
        .get("__rpi")
        .and_then(|context| context.get("runtimeHandle"))
        .and_then(serde_json::Value::as_u64)
    else {
        return "fixture runtime handle is unavailable".to_owned();
    };
    let Ok(handle) = usize::try_from(handle) else {
        return "fixture runtime handle exceeds this platform's pointer width".to_owned();
    };
    let payload = request
        .get("hostComponent")
        .cloned()
        .unwrap_or(serde_json::Value::Null)
        .to_string();
    let mut response = StbString::empty();
    let status = action(
        RuntimeActionId::UiComponent.into(),
        StbStringRef::from_str(&payload),
        &raw mut response,
        handle as *mut c_void,
    );
    // SAFETY: the host writes a uniquely owned response through the output slot.
    let response = unsafe { OwnedString::from_raw(response, Some(free_string)) };
    if status == 0 {
        format!("host component result: {}", response.to_string_lossy())
    } else {
        format!("host component denied: {}", response.to_string_lossy())
    }
}

/// Completes one fixture tool invocation with deterministic JSON content.
///
/// # Parameters
/// - `handle`: Opaque state allocated by `fixture_tool_execute`.
/// - `_partial_cb`: Optional progress callback, unused by this fixture.
/// - `_user_data`: Host callback context, unused by this fixture.
///
/// # Returns
/// A terminal tool result containing fixture text.
pub(super) extern "C" fn fixture_tool_poll(
    handle: StepHandle,
    _partial_cb: Option<ToolPartialCb>,
    _user_data: *mut c_void,
) -> StepResult {
    let Some(result) = (unsafe { handle.cast::<String>().as_ref() }) else {
        return StepResult::err(StbString::from_string(
            "invalid fixture tool state".to_owned(),
        ));
    };
    let content = serde_json::json!({"content":[{"type":"text","text":result}]});
    StepResult::done(StbString::from_string(content.to_string()))
}

/// Handles cancellation for the immediately completing fixture tool.
///
/// # Parameters
/// - `_handle`: Opaque tool state, unused by this fixture.
pub(super) extern "C" fn fixture_tool_cancel(_handle: StepHandle) {}

/// Frees the opaque state allocated for a fixture tool invocation.
///
/// # Parameters
/// - `handle`: Opaque state allocated by `fixture_tool_execute`.
pub(super) extern "C" fn fixture_tool_destroy(handle: StepHandle) {
    if !handle.is_null() {
        // SAFETY: execute allocates this exact unit value and destroy runs once.
        drop(unsafe { Box::from_raw(handle.cast::<()>()) });
    }
}
