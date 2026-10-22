//! Verifies native TUI workflows through a real pseudo-terminal.

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
#[path = "tui_pty/plan_view.rs"]
mod tui_pty_plan_view;
mod tui_pty_support;

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
    tui.send(b"\x1b");
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/edit\r");
    assert!(tui.read_until("Edit source"), "{}", tui.diagnostic());
    tui.send(b"left.saturating_add(right)\r");
    assert!(tui.read_until("Change review"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("left.saturating_add(right)"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.screen().contains("validation: Passed"));
    assert!(tui.source().contains("left + right"));
    tui.send(b"\r");
    assert!(tui.read_until("Enter confirm"), "{}", tui.diagnostic());
    let confirmation = tui.screen();
    assert!(confirmation.contains("Reviewed base:"), "{confirmation}");
    assert!(confirmation.contains("Candidate result:"), "{confirmation}");
    tui.send(b"\r");
    assert!(tui.read_until("materialized"), "{}", tui.diagnostic());
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
    tui.send(b"\x1b");
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/edit\r");
    assert!(tui.read_until("Edit source"), "{}", tui.diagnostic());
    tui.send(b"left.saturating_add(right)\r");
    assert!(tui.read_until("validation: Passed"), "{}", tui.diagnostic());
    tui.external_edit();
    tui.send(b"\r\r");
    assert!(
        tui.read_until("filesystem_conflict"),
        "{}",
        tui.diagnostic()
    );
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
