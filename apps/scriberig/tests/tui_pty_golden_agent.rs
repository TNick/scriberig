//! Verifies the native TUI semantic-agent golden path.

#[allow(dead_code)]
mod tui_pty_support;

use tui_pty_support::{TuiPty, test_lock};

/// TUI-GOLDEN-05 runs the deterministic agent through exact semantic scope.
#[test]
fn tui_golden_05_agent_uses_exact_semantic_scope_and_returns_compact_result() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send(b"Inspect the existing source structure.\r");
    assert!(tui.read_until("semantic_find"), "{}", tui.diagnostic());
    assert!(!tui.screen().contains("I inspected the source structure"));
    assert!(tui.read_until("semantic_render"), "{}", tui.diagnostic());
    assert!(!tui.screen().contains("I inspected the source structure"));
    assert!(
        tui.read_until("I inspected the source structure"),
        "{}",
        tui.diagnostic()
    );
    let screen = tui.screen();
    assert!(screen.contains("semantic_find"), "{}", tui.diagnostic());
    assert!(screen.contains("semantic_render"), "{}", tui.diagnostic());
    assert!(tui.source().contains("left + right"));
    assert!(tui.shutdown());
}
