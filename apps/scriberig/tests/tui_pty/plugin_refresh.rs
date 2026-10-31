//! Verifies provider-package commands survive repeated TUI refreshes.

use super::tui_pty_conversation::seed_named_session;
use super::tui_pty_support::{TuiPty, test_lock};

/// Verifies repeated provider refreshes leave one plugin command in the live palette.
#[cfg(feature = "provider-package-worker-tests")]
#[test]
fn tui_pty_refreshes_plugin_command_registrations_without_duplicates() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_provider_package();

    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    let session = seed_named_session(&tui, "Plugin refresh conversation");
    tui.send_slow(format!("/switch {session}\r").as_bytes());
    assert!(
        tui.read_until("Plugin refresh conversation"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    tui.send_slow(b"scriberig-fixture-command");
    assert!(
        tui.read_until("scriberig-fixture-command"),
        "{}",
        tui.diagnostic()
    );
    assert!(
        tui.read_until("Runs the isolated ScribeRig fixture command."),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(
        tui.read_until("fixture command received:"),
        "{}",
        tui.diagnostic()
    );

    tui.send_slow(b"/model\r");
    assert!(tui.read_until("Select model"), "{}", tui.diagnostic());
    tui.send_slow(b"scriberig-fixture-model");
    tui.send(b"\r");
    assert!(
        tui.read_until("model scriberig-fixture-model"),
        "{}",
        tui.diagnostic()
    );
    tui.send_slow(b"first provider turn\r");
    assert!(
        tui.read_until("isolated fixture provider observed tool result"),
        "{}",
        tui.diagnostic()
    );
    tui.send_slow(b"second provider turn\r");
    assert!(
        tui.read_until_next("isolated fixture provider observed tool result"),
        "{}",
        tui.diagnostic()
    );

    tui.send_slow(b"/model\r");
    assert!(tui.read_until("Select model"), "{}", tui.diagnostic());
    tui.send_slow(b"scriberig-fixture-model");
    assert!(
        tui.read_until("ScribeRig Isolated Fixture"),
        "{}",
        tui.diagnostic()
    );
    assert_eq!(
        tui.screen().matches("ScribeRig Isolated Fixture").count(),
        1,
        "refreshed model selector contains a duplicate package model: {}",
        tui.diagnostic()
    );
    tui.send(b"\x1b");
    assert!(tui.read_until_not("Select model"), "{}", tui.diagnostic());

    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    tui.send_slow(b"scriberig-fixture-command");
    assert!(
        tui.read_until("scriberig-fixture-command"),
        "{}",
        tui.diagnostic()
    );
    assert_eq!(
        tui.screen().matches("scriberig-fixture-command").count(),
        1,
        "refreshed command palette contains duplicate package commands: {}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(
        tui.read_until("fixture command received:"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}
