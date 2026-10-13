//! Verifies native TUI workflows through a real pseudo-terminal.

#[path = "tui_pty_support/lifecycle_test.rs"]
mod tui_pty_lifecycle;
mod tui_pty_support;

use portable_pty::PtySize;
use tui_pty_support::{TuiPty, test_lock};

/// Verifies a real TUI process starts with a fresh database and no project.
#[test]
fn tui_pty_starts_without_a_project() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_empty();

    assert!(
        tui.read_until("project: unselected"),
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

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    let screen = tui.screen();
    assert!(screen.contains("Explorer"));
    assert!(screen.contains("Primary"));
    assert!(screen.contains("Inspector"));
    assert!(tui.shutdown());
}

/// Verifies project-relative paths and semantic kinds are visible in Explorer.
#[test]
fn tui_pty_shows_readable_explorer_rows() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b"\t\t\r");
    assert!(tui.read_until("README.md"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("README.md"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("project.source"),
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

    assert!(tui.read_until("Entity:"), "{}", tui.diagnostic());
    tui.send(b"\t\t\r");
    assert!(tui.read_until("Calculator"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("add"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// TUI-GOLDEN-02 opens the real source projection for the imported file.
#[test]
fn tui_golden_02_shows_rust_source_projection() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();
    assert!(tui.read_until("Entity:"), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Open help"), "{}", tui.diagnostic());
    for (partial, character) in [
        ("p", b'p'),
        ("pr", b'r'),
        ("pro", b'o'),
        ("proj", b'j'),
        ("proje", b'e'),
        ("projec", b'c'),
        ("project", b't'),
        ("projecti", b'i'),
        ("projectio", b'o'),
        ("projection", b'n'),
    ] {
        tui.send(&[character]);
        assert!(
            tui.read_until(&format!("Search: {partial}")),
            "{}",
            tui.diagnostic()
        );
    }
    tui.send(b"\r");
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
    assert!(tui.read_until("Entity:"), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"projection\r");
    assert!(tui.read_until("left + right"), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"edit\r");
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
    tui.send(b"\r");
    assert!(tui.read_until("materialized"), "{}", tui.diagnostic());
    assert!(tui.source().contains("left.saturating_add(right)"));
    let fixture = tui.quit_retain_fixture();
    let mut restarted = TuiPty::restart(fixture.clone());
    assert!(
        restarted.read_until("Entity:"),
        "{}",
        restarted.diagnostic()
    );
    restarted.send(b":");
    assert!(
        restarted.read_until("Search:"),
        "{}",
        restarted.diagnostic()
    );
    restarted.send(b"projection\r");
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
    assert!(tui.read_until("Entity:"), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"projection\r");
    assert!(tui.read_until("left + right"), "{}", tui.diagnostic());
    tui.send(b":edit\r");
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

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b"?");
    assert!(tui.read_until("Ctrl-C"));
    let screen = tui.screen();
    assert!(screen.contains("Overlay"));
    for label in [
        "Ctrl-C",
        "Esc",
        "Tab",
        "Shift-Tab",
        "Enter",
        "Up",
        "Down",
        "Left",
        "Backspace",
        ":",
        "/",
        "?",
        "r",
        "q",
    ] {
        assert!(screen.contains(label), "missing help key {label}: {screen}");
    }
    assert!(tui.shutdown());
}

/// Verifies the command palette opens and closes through terminal input.
#[test]
fn tui_pty_opens_and_closes_command_palette() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"));
    tui.send(b"\x1b");
    assert!(tui.read_until_not("Search:"));
    assert!(tui.shutdown());
}

/// Verifies a native terminal resize preserves the selected project context.
#[test]
fn tui_pty_resizes_without_losing_project_context() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "));
    tui.resize(PtySize {
        rows: 30,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    });
    assert!(tui.read_until_next("project: "));
    assert!(tui.screen().contains("Primary"));
    assert!(tui.shutdown());
}

/// Verifies the browse focus cycle and reverse focus binding in a live TUI.
#[test]
fn tui_pty_cycles_focus_between_panes() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Primary [ACTIVE]"), "{}", tui.diagnostic());
    tui.send(b"\t");
    assert!(
        tui.read_until_next("Inspector [ACTIVE]"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\t");
    assert!(
        tui.read_until_next("Explorer [ACTIVE]"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[Z");
    assert!(
        tui.read_until_next("Inspector [ACTIVE]"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}

/// Verifies browse commands leave the live semantic shell usable.
#[test]
fn tui_pty_accepts_browse_commands() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Entity:"), "{}", tui.diagnostic());
    tui.send(b"r\r\x1b[A\x1b[B\x1b[D\x1b[C\x08x");
    assert!(tui.read_until_next("Entity:"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("project: "));
    assert!(tui.shutdown());
}

/// Verifies palette cursor movement through both horizontal and vertical keys.
#[test]
fn tui_pty_moves_palette_selection() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"));
    tui.send(b"\x1b[B\x1b[C\x1b[A\x1b[D");
    assert!(tui.read_until_next("Search:"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Verifies every always-available local palette command is discoverable.
#[test]
fn tui_pty_lists_local_palette_commands() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    let screen = tui.screen();
    for command in [
        "Open help",
        "Refresh",
        "Go back",
        "Quit",
        "View overview",
        "View operations",
    ] {
        assert!(screen.contains(command), "missing {command}: {screen}");
    }
    tui.send(b"\x1b");
    assert!(tui.read_until_not("Search:"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Verifies find input accepts text, handles backspace, and submits.
#[test]
fn tui_pty_submits_find_query() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b"/");
    assert!(tui.read_until("Find:"), "{}", tui.diagnostic());
    tui.send(b"readmx");
    tui.send(&[8]);
    tui.send(b"e");
    assert!(tui.read_until("Find: readme"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(tui.screen().contains("Find: readme"));
    assert!(tui.shutdown());
}

/// Verifies palette selection, activation, and view routing in the live TUI.
#[test]
fn tui_pty_activates_palette_commands() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"));
    tui.send(b"operations");
    tui.send(b"\r");
    assert!(
        tui.read_until("No observed operations"),
        "{}",
        tui.diagnostic()
    );

    tui.send(b":");
    assert!(tui.read_until("Search:"));
    tui.send(b"overview");
    tui.send(b"\r");
    assert!(tui.read_until("Entity:"), "{}", tui.diagnostic());

    assert!(tui.shutdown());
}

/// Verifies history loading and the agent composer command from the palette.
#[test]
fn tui_pty_opens_history_and_agent_composer() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"));
    tui.send(b"history");
    assert!(tui.read_until("Search: history"));
    tui.send(b"\r");
    assert!(tui.read_until("History @"), "{}", tui.diagnostic());

    tui.send(b":");
    assert!(tui.read_until("Search:"));
    tui.send(b"start-agent");
    assert!(tui.read_until("Search: start-agent"));
    tui.send(b"\r");
    assert!(tui.read_until("Session:"), "{}", tui.diagnostic());
    tui.send(b"\x1b");
    assert!(tui.read_until_not("Session:"));
    assert!(tui.shutdown());
}
