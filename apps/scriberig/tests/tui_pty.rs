//! Verifies native TUI workflows through a real pseudo-terminal.

mod tui_pty_support;

use portable_pty::PtySize;
use tui_pty_support::{TuiPty, test_lock};

/// Verifies the native TUI project context is visible after startup.
#[test]
fn tui_pty_shows_project_context() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: prj_"), "{}", tui.diagnostic());
    let screen = tui.screen();
    assert!(screen.contains("Explorer"));
    assert!(screen.contains("Primary"));
    assert!(screen.contains("Inspector"));
    assert!(tui.shutdown());
}

/// Verifies the help overlay is reachable from the running native TUI.
#[test]
fn tui_pty_opens_help_overlay() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: prj_"), "{}", tui.diagnostic());
    tui.send(b"?");
    assert!(tui.read_until("Ctrl-C"));
    assert!(tui.screen().contains("Overlay"));
    assert!(tui.shutdown());
}

/// Verifies the command palette opens and closes through terminal input.
#[test]
fn tui_pty_opens_and_closes_command_palette() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: prj_"), "{}", tui.diagnostic());
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

    assert!(tui.read_until("project: prj_"));
    tui.resize(PtySize {
        rows: 30,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    });
    assert!(tui.read_until_next("project: prj_"));
    assert!(tui.screen().contains("Primary"));
    assert!(tui.shutdown());
}

/// Verifies the browse focus cycle and reverse focus binding in a live TUI.
#[test]
fn tui_pty_cycles_focus_between_panes() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("focus: Primary"), "{}", tui.diagnostic());
    tui.send(b"\t");
    assert!(
        tui.read_until_next("focus: Inspector"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\t");
    assert!(
        tui.read_until_next("focus: Explorer"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[Z");
    assert!(
        tui.read_until_next("focus: Inspector"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}

/// Verifies every always-available local palette command is discoverable.
#[test]
fn tui_pty_lists_local_palette_commands() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: prj_"), "{}", tui.diagnostic());
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

    assert!(tui.read_until("project: prj_"), "{}", tui.diagnostic());
    tui.send(b"/");
    assert!(tui.read_until("Find:"), "{}", tui.diagnostic());
    tui.send(b"readmx");
    tui.send(&[8]);
    tui.send(b"e");
    assert!(tui.read_until("Find: readme"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(tui.read_until_not("OVERLAY"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("Find: readme"));
    assert!(tui.shutdown());
}

/// Verifies palette selection, activation, and view routing in the live TUI.
#[test]
fn tui_pty_activates_palette_commands() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: prj_"), "{}", tui.diagnostic());
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
