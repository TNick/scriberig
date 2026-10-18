//! Verifies PHP implementation cancellation is durable and isolated.

#[allow(dead_code)]
#[path = "support/tui_pty_self_hosting_discovery_support.rs"]
mod discovery_support;

use std::fs;

use discovery_support::{
    assert_project_and_checkout_unchanged, cancel_active_agent, onboard_project,
    submit_implementation, test_lock, wait_for_cancellation,
};
use scriberig_session::BranchStatus;

/// Cancels implementation after bounded B1 discovery and before candidate creation.
#[test]
fn tui_cancels_php_implementation_without_project_or_source_mutation() {
    let _lock = test_lock();
    let mut evidence = onboard_project(true);
    submit_implementation(&mut evidence.tui, evidence.session, &evidence.database);
    cancel_active_agent(&mut evidence.tui);
    let saved = wait_for_cancellation(&evidence.database, evidence.session);
    let implementation = saved
        .branches
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose.contains("implement PHP ingestion"))
        })
        .expect("cancelled implementation branch");
    assert_eq!(implementation.status, BranchStatus::Abandoned);
    assert_eq!(implementation.tool_invocations.len(), 0);
    assert_project_and_checkout_unchanged(&evidence);
    let fixture = evidence.tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove cancellation fixture");
}
