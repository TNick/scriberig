//! Branch cancellation reaching one in-flight plugin worker call.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use scriberig_agent_runtime::plugin_worker::PluginToolBridge;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use super::fixture::{FixtureRoot, PLUGIN_CAPABILITY, WAIT_TOOL, start_worker};
use super::helpers::{execute, granted, request, request_with, waited_for_marker};

/// Verifies a branch cancellation reaches the worker process and settles as an
/// upstream abort instead of a provider or agent failure.
#[test]
fn branch_cancellation_aborts_an_in_flight_worker_call() {
    let root = FixtureRoot::new("cancellation");
    let marker = root.plugin_dir.join("cancelled");
    let worker = Arc::new(
        start_worker(&[WAIT_TOOL], &[WAIT_TOOL], &root, Some(&marker), true)
            .expect("authorized fixture worker"),
    );
    let bridge =
        PluginToolBridge::new(Arc::clone(&worker), PLUGIN_CAPABILITY).expect("attested bridge");
    let records = Arc::new(Mutex::new(Vec::new()));
    let composed = bridge
        .compose(request_with(granted(), Arc::clone(&records)))
        .expect("compose");

    let cancellation = CancellationToken::new();
    let delayed = cancellation.clone();
    let watcher = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        delayed.cancel();
    });
    let started = Instant::now();
    let error = execute(&composed.tools()[0], json!({}), cancellation.clone())
        .expect_err("a cancelled worker call must not settle as success");
    let elapsed = started.elapsed();
    watcher.join().expect("cancellation watcher");

    assert!(
        error.is_abort(),
        "a cancelled worker call must be an abort: {error}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "cancellation waited for the request deadline instead: {elapsed:?}"
    );
    assert!(
        waited_for_marker(&marker),
        "the worker never received the cancel frame"
    );

    let records = records.lock().expect("evidence records");
    assert_eq!(records.len(), 1);
    assert!(
        records[0].failed,
        "the cancelled call is recorded as failed"
    );
    assert_eq!(records[0].name, WAIT_TOOL);

    root.remove();
}

/// Verifies a branch that is already cancelled never dispatches a worker call.
#[test]
fn an_already_cancelled_branch_never_dispatches_a_worker_call() {
    let root = FixtureRoot::new("cancelled-first");
    let marker = root.plugin_dir.join("cancelled");
    let worker = Arc::new(
        start_worker(&[WAIT_TOOL], &[WAIT_TOOL], &root, Some(&marker), true)
            .expect("authorized fixture worker"),
    );
    let bridge =
        PluginToolBridge::new(Arc::clone(&worker), PLUGIN_CAPABILITY).expect("attested bridge");
    let composed = bridge.compose(request(granted())).expect("compose");

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let error = execute(&composed.tools()[0], json!({}), cancellation)
        .expect_err("a cancelled branch must not dispatch");
    assert!(error.is_abort(), "a cancelled branch must abort: {error}");
    assert!(
        !marker.exists(),
        "a cancelled branch dispatched a worker call anyway"
    );

    root.remove();
}
