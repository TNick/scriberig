//! Verifies post-onboarding architecture branch cancellation is durable and isolated.

/// Cancels a read-only architecture child after onboarding a real copied checkout.
#[test]
fn tui_cancels_php_discovery_without_project_or_source_mutation() {
    let _lock = test_lock();
    let mut evidence = onboard_project(false);
    submit_discovery(&mut evidence.tui, evidence.session, &evidence.database);
    cancel_active_agent(&mut evidence.tui);
    let saved = wait_for_cancellation(&evidence.database, evidence.session);
    let discovery = saved
        .branches
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose.contains("determine minimal extension points"))
        })
        .expect("cancelled child branch");
    assert_eq!(discovery.status, BranchStatus::Abandoned);
    assert!(saved.branches.iter().any(|branch| {
        branch
            .result
            .as_ref()
            .is_some_and(|result| result.result_kind.as_deref() == Some("agent-cancelled"))
    }));
    assert_eq!(
        ProjectService::new(std::sync::Arc::new(
            SqliteStore::open(&evidence.database).expect("reopen project store"),
        ))
        .workspace(evidence.project)
        .expect("project remains active")
        .snapshot,
        evidence.base
    );
    assert_eq!(
        fs::read(evidence.checkout.join("Cargo.toml")).expect("manifest"),
        evidence.manifest
    );
    assert_eq!(
        fs::read(
            evidence
                .checkout
                .join("crates/scriberig-project/src/scan.rs")
        )
        .expect("scanner"),
        evidence.scanner
    );
    assert!(
        !evidence
            .checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
    let fixture = evidence.tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove cancellation fixture");
}

#[allow(dead_code)]
#[path = "support/tui_pty_self_hosting_discovery_support.rs"]
mod discovery_support;

use std::fs;

use discovery_support::{
    cancel_active_agent, onboard_project, submit_discovery, test_lock, wait_for_cancellation,
};
use scriberig_project::ProjectService;
use scriberig_session::BranchStatus;
use scriberig_store_sqlite::SqliteStore;
