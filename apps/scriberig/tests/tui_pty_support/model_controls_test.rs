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
    assert!(
        tui.read_until("Select provider model"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.screen().contains("Fake A"), "{}", tui.diagnostic());
    tui.send(b"\x1b");
    std::thread::sleep(std::time::Duration::from_millis(100));

    tui.send(b"\x1bOR");
    assert!(tui.read_until("Select model"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("Fake A"), "{}", tui.diagnostic());
    tui.send(b"\x1b");
    std::thread::sleep(std::time::Duration::from_millis(100));

    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("plans"), "{}", tui.diagnostic());
    tui.send(b"\x1b");
    std::thread::sleep(std::time::Duration::from_millis(100));

    tui.send(b"\x1bOP");
    assert!(tui.read_until("ScribeRig help"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("Composer editing"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b");
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());

    tui.send(b"\x1bOS");
    assert!(
        tui.read_until("no available thinking levels"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}
