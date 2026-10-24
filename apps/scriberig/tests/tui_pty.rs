//! Verifies native TUI workflows through a real pseudo-terminal.

use std::time::{Duration, Instant};

#[path = "tui_pty_support/bootstrap_test.rs"]
mod tui_pty_bootstrap;
#[path = "tui_pty/branch.rs"]
mod tui_pty_branch;
#[path = "tui_pty/conversation.rs"]
mod tui_pty_conversation;
#[path = "tui_pty_support/lifecycle_test.rs"]
mod tui_pty_lifecycle;
#[path = "tui_pty_support/model_controls_test.rs"]
mod tui_pty_model_controls;
#[path = "tui_pty/plan_relations.rs"]
mod tui_pty_plan_relations;
#[path = "tui_pty/plan_view.rs"]
mod tui_pty_plan_view;
mod tui_pty_support;
#[path = "tui_pty/tree_identity.rs"]
mod tui_pty_tree_identity;

use portable_pty::PtySize;
use tui_pty_support::{TuiPty, test_lock};

/// Verifies a real TUI process starts with a fresh database and no project.
#[test]
fn tui_pty_starts_without_a_project() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_empty();

    assert!(
        tui.read_until("Type a message to start a conversation."),
        "{}",
        tui.diagnostic()
    );
    tui.send_slow(b"/project\r");
    assert!(
        tui.read_until("No active project is selected."),
        "{}",
        tui.diagnostic()
    );
    assert!(!tui.screen().contains("Entity:"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Verifies the native TUI project context is visible after startup.
#[test]
fn tui_pty_shows_project_context() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    let screen = tui.screen();
    assert!(screen.contains("Conversation"));
    assert!(screen.contains("Type a message to start a conversation."));
    assert!(screen.contains("Enter send"));
    assert!(tui.shutdown());
}

/// Verifies project-relative paths and semantic kinds are visible in Explorer.
#[test]
fn tui_pty_shows_readable_explorer_rows() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/explorer\r");
    assert!(tui.read_until("README.md"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("README.md"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("Semantic explorer"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}

/// TUI-GOLDEN-01 opens the copied Rust project and exposes named symbols.
#[test]
fn tui_golden_01_shows_named_rust_symbols() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/explorer\r");
    assert!(tui.read_until("Calculator"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("add"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// TUI-GOLDEN-02 opens the real source projection for the imported file.
#[test]
fn tui_golden_02_shows_rust_source_projection() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/projection\r");
    assert!(tui.read_until("src/lib.rs"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("left + right"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}

/// TUI-GOLDEN-03 edits, reviews, validates, applies, and persists a candidate.
#[test]
fn tui_golden_03_edits_reviews_applies_and_restarts() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/projection\r");
    assert!(tui.read_until("left + right"), "{}", tui.diagnostic());
    show_conversation(&mut tui);
    tui.send_slow(b"/edit\r");
    assert!(tui.read_until("Edit source"), "{}", tui.diagnostic());
    tui.send_slow(b"left.saturating_add(right)\r");
    assert!(tui.read_until("Candidate review"), "{}", tui.diagnostic());
    assert!(tui.read_until("Validation: Passed"), "{}", tui.diagnostic());
    assert!(tui.source().contains("left + right"));
    tui.send(b"\r");
    assert!(
        tui.read_until("Confirmation: required"),
        "{}",
        tui.diagnostic()
    );
    let confirmation = tui.screen();
    assert!(confirmation.contains("Base snapshot:"), "{confirmation}");
    assert!(confirmation.contains("Result snapshot:"), "{confirmation}");
    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    tui.send_slow(b"Confirm");
    assert!(
        tui.read_until("Confirm reviewed change"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(
        tui.read_until("Action: commit change"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !tui.source().contains("left.saturating_add(right)") {
        assert!(
            Instant::now() < deadline,
            "candidate was not materialized: {}",
            tui.diagnostic()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(tui.source().contains("left.saturating_add(right)"));
    let fixture = tui.quit_retain_fixture();
    let mut restarted = TuiPty::restart(fixture.clone());
    assert!(
        restarted.read_until("Conversation"),
        "{}",
        restarted.diagnostic()
    );
    restarted.send_slow(b"/projection\r");
    assert!(
        restarted.read_until("left.saturating_add(right)"),
        "{}",
        restarted.diagnostic()
    );
    assert!(restarted.shutdown());
}

/// TUI-GOLDEN-04 preserves an external edit instead of silently overwriting it.
#[test]
fn tui_golden_04_external_edit_is_not_overwritten() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/projection\r");
    assert!(tui.read_until("left + right"), "{}", tui.diagnostic());
    show_conversation(&mut tui);
    tui.send_slow(b"/edit\r");
    assert!(tui.read_until("Edit source"), "{}", tui.diagnostic());
    tui.send_slow(b"left.saturating_add(right)\r");
    assert!(tui.read_until("Validation: Passed"), "{}", tui.diagnostic());
    tui.external_edit();
    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    tui.send_slow(b"Confirm");
    assert!(
        tui.read_until("Confirm reviewed change"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(
        tui.read_until("Action: commit change"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !tui.source().contains("external edit") {
        assert!(
            Instant::now() < deadline,
            "external edit was not preserved: {}",
            tui.diagnostic()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(tui.source().contains("external edit"));
    assert!(!tui.source().contains("left.saturating_add(right)"));
    let _ = tui.shutdown_allow_failure();
}

/// Verifies the help overlay is reachable from the running native TUI.
#[test]
fn tui_pty_opens_help_overlay() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"?");
    assert!(tui.read_until("ScribeRig help"), "{}", tui.diagnostic());
    assert!(
        tui.read_until("Delete character backward"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}

/// Verifies a local slash command opens and closes its auxiliary view.
#[test]
fn tui_pty_opens_and_closes_command_palette() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"?");
    assert!(tui.read_until("ScribeRig help"));
    tui.send(b"\x1b");
    assert!(tui.read_until_not("ScribeRig help"));
    assert!(tui.shutdown());
}

/// Verifies a native terminal resize preserves the selected project context.
#[test]
fn tui_pty_resizes_without_losing_project_context() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"));
    tui.resize(PtySize {
        rows: 30,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    });
    assert!(tui.read_until("Conversation"));
    assert!(tui.screen().contains("Conversation"));
    assert!(tui.shutdown());
}

/// Verifies browse mode can return to the ordinary message composer.
#[test]
fn tui_pty_browse_returns_to_composer() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send(b"\x1b[18~");
    assert!(
        tui.read_until_next("Enter pin reply target"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[18~");
    assert!(tui.read_until_next("Enter send"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Verifies ordinary text immediately enters the persistent composer.
#[test]
fn tui_pty_types_immediately() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send(b"hello");
    assert!(tui.read_until_next("hello"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Switches the live TUI back to the Conversation view through the palette.
///
/// - `tui`: Running process whose current primary view may be a semantic view.
fn show_conversation(tui: &mut TuiPty) {
    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    tui.send_slow(b"conversation\r");
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    std::thread::sleep(std::time::Duration::from_millis(500));
}
