//! Verifies upstream provider, model, and thinking controls in the live TUI.

use super::{TuiPty, test_lock};

/// Verifies the active terminal path shows upstream selectors for all controls.
#[test]
fn tui_pty_uses_upstream_model_controls() {
    let _lock = test_lock();
    let catalog = r#"{"providers":{"local":{"api":"openai-completions","models":[{"id":"fake-a","name":"Fake A"}]}}}"#;
    let mut tui = TuiPty::start_with_model_catalog(catalog);

    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("F(2) Provider"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1bOQ");
    assert!(tui.read_until("local"), "{}", tui.diagnostic());
    tui.send(b"\x1b[B");
    tui.send(b"\r");
    assert!(tui.read_until("Fake A"), "{}", tui.diagnostic());
    tui.send(b"\x1b");
    assert!(
        tui.read_until_not("Select provider model"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}
