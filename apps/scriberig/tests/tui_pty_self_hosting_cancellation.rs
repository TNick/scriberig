//! Verifies cancellation of native project onboarding leaves no hidden effects.

#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::time::{Duration, Instant};

use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::TuiPty;

/// Verifies an active native onboarding operation can be cancelled safely.
#[test]
fn tui_cancellation_persists_abandoned_branch_without_project_or_source_mutation() {
    let mut tui = TuiPty::start_self_hosting_cancellation();
    assert!(
        tui.read_until("project: unselected"),
        "{}",
        tui.diagnostic()
    );
    let checkout = tui.fixture_path().join("scriberig");
    let manifest = fs::read(checkout.join("Cargo.toml")).expect("baseline manifest");
    let scanner =
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("baseline scanner");

    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"start-agent\r");
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    assert!(tui.read_until("Session:"), "{}", tui.diagnostic());
    let session = visible_session(&tui.screen());
    tui.send(b"Onboard the current project.\r");
    wait_for_provider_gate(&tui);

    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"operations");
    assert!(tui.read_until("> View operations"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(
        tui.read_until_for("Running", Duration::from_secs(10)),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"x");
    assert!(
        tui.read_until_for("Cancelled", Duration::from_secs(10)),
        "{}",
        tui.diagnostic()
    );
    tui.release_agent_provider();

    assert!(
        !checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
    assert_eq!(
        fs::read(checkout.join("Cargo.toml")).expect("manifest"),
        manifest
    );
    assert_eq!(
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("scanner"),
        scanner
    );
    assert!(
        SqliteStore::open(tui.fixture_path().join("scriberig.sqlite"))
            .expect("store")
            .list_projects()
            .expect("project list")
            .is_empty(),
        "cancelled onboarding imported a project"
    );

    let database = tui.fixture_path().join("scriberig.sqlite");
    let session_store = SqliteSessionStore::open(&database).expect("session store");
    let deadline = Instant::now() + Duration::from_secs(15);
    let saved = loop {
        let saved = session_store
            .load(session)
            .expect("load session")
            .expect("durable session");
        if saved.branches.iter().any(|branch| {
            branch
                .result
                .as_ref()
                .is_some_and(|result| result.result_kind.as_deref() == Some("agent-cancelled"))
        }) {
            break saved;
        }
        assert!(
            Instant::now() < deadline,
            "cancelled branch result was not persisted"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let branch = saved
        .branches
        .iter()
        .find(|branch| {
            branch
                .result
                .as_ref()
                .is_some_and(|result| result.result_kind.as_deref() == Some("agent-cancelled"))
        })
        .expect("durable cancellation result");
    assert!(
        branch
            .outcome
            .as_deref()
            .is_some_and(|outcome| outcome.contains("cancel"))
    );

    let fixture = tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove cancellation fixture");
}

/// Waits until the deterministic provider is blocked before opening Operations.
///
/// # Parameters
/// - `tui`: Active native TUI process owning the provider gate.
///
/// # Panics
/// Panics when the provider never reaches its wait marker or already received release.
fn wait_for_provider_gate(tui: &TuiPty) {
    let waiting = tui.fixture_path().join("agent-provider.waiting");
    let release = tui.fixture_path().join("agent-provider.release");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !waiting.is_file() {
        assert!(
            Instant::now() < deadline,
            "provider did not reach its cancellation barrier"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!release.exists(), "provider release marker already exists");
}

/// Reads the durable session identity displayed by the agent composer.
///
/// # Parameters
/// - `screen`: Current native TUI screen.
///
/// # Returns
/// The session identity rendered by the composer.
fn visible_session(screen: &str) -> scriberig_core::EntityId {
    screen
        .split("Session:")
        .nth(1)
        .and_then(|text| text.split_whitespace().next())
        .expect("session visible in composer")
        .parse()
        .expect("session identity")
}
