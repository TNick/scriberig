//! Verifies the native TUI semantic-agent golden path.

#[allow(dead_code)]
mod tui_pty_support;

use tui_pty_support::{TuiPty, test_lock};

/// TUI-GOLDEN-05 runs the deterministic agent through semantic scope and review.
#[test]
fn tui_golden_05_agent_uses_selected_scope_and_reviews_candidate() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();
    assert!(tui.read_until("Entity:"), "{}", tui.diagnostic());
    tui.send(b":project-agent\r");
    assert!(tui.read_until("Change review"), "{}", tui.diagnostic());
    let screen = tui.screen();
    assert!(screen.contains("scope: entity="), "{}", tui.diagnostic());
    assert!(screen.contains("base: snp_"), "{}", tui.diagnostic());
    assert!(screen.contains("left.saturating_add(right)"));
    assert!(tui.source().contains("left + right"));
    assert!(tui.shutdown());
}
