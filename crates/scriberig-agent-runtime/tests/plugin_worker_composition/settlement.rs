//! Settlement of one composed plugin worker call into provider-visible output
//! and durable `ScribeRig` tool evidence.

use std::sync::{Arc, Mutex};

use rpi_agent::TextContentOrImage;
use scriberig_agent_runtime::plugin_worker::PluginToolBridge;
use scriberig_session::ToolActivityKind;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use super::fixture::{ECHO_TOOL, FAIL_TOOL, FixtureRoot, PLUGIN_CAPABILITY, start_worker};
use super::helpers::{MAX_PROVIDER_RESULT_BYTES, execute, granted, request_with};

/// Verifies one settled worker call returns a bounded provider projection and
/// exactly one durable evidence record.
#[test]
fn settled_worker_call_is_bounded_and_recorded_once() {
    let root = FixtureRoot::new("settlement");
    let worker = Arc::new(
        start_worker(&[ECHO_TOOL], &[ECHO_TOOL], &root, None, true)
            .expect("authorized fixture worker"),
    );
    let bridge =
        PluginToolBridge::new(Arc::clone(&worker), PLUGIN_CAPABILITY).expect("attested bridge");
    let records = Arc::new(Mutex::new(Vec::new()));
    let composed = bridge
        .compose(request_with(granted(), Arc::clone(&records)))
        .expect("compose");

    let big = "x".repeat(20_000);
    let result = execute(
        &composed.tools()[0],
        json!({"payload": big}),
        CancellationToken::new(),
    )
    .expect("authorized worker call settles");

    assert_eq!(result.content.len(), 1, "provider-visible text only");
    let TextContentOrImage::Text(text) = &result.content[0] else {
        panic!("the provider projection must be text");
    };
    assert!(
        text.text.len() <= MAX_PROVIDER_RESULT_BYTES,
        "provider-visible result exceeded the bound: {} bytes",
        text.text.len()
    );
    assert!(
        result.details.is_null() && result.added_tool_names.is_empty(),
        "structured details and dynamic tool additions must not cross the adapter"
    );

    let records = records.lock().expect("evidence records");
    assert_eq!(records.len(), 1, "exactly one settled call is recorded");
    let record = &records[0];
    assert_eq!(
        record.ordinal, 0,
        "the branch activity sequence orders the call"
    );
    assert_eq!(record.name, ECHO_TOOL);
    assert_eq!(record.category, Some(ToolActivityKind::OpaqueFallback));
    assert!(!record.failed);
    assert!(record.child_branch.is_none());

    root.remove();
}

/// Verifies a worker-side failure stays a bounded tool error and is recorded as
/// a failed call rather than a semantic success.
#[test]
fn worker_failure_is_a_bounded_failed_tool_record() {
    let root = FixtureRoot::new("failure");
    let worker = Arc::new(
        start_worker(&[FAIL_TOOL], &[FAIL_TOOL], &root, None, true)
            .expect("authorized fixture worker"),
    );
    let bridge =
        PluginToolBridge::new(Arc::clone(&worker), PLUGIN_CAPABILITY).expect("attested bridge");
    let records = Arc::new(Mutex::new(Vec::new()));
    let composed = bridge
        .compose(request_with(granted(), Arc::clone(&records)))
        .expect("compose");

    let error = execute(&composed.tools()[0], json!({}), CancellationToken::new())
        .expect_err("the worker failure must surface");
    assert!(!error.is_abort(), "a worker failure is not a cancellation");
    assert!(
        error.to_string().contains("fixture tool failure"),
        "safe worker failure text was lost: {error}"
    );

    let records = records.lock().expect("evidence records");
    assert_eq!(records.len(), 1);
    assert!(
        records[0].failed,
        "the settled failure is recorded as failed"
    );
    assert_eq!(records[0].name, FAIL_TOOL);

    root.remove();
}
