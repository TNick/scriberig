//! Verifies the real native TUI opens a mixed rich project through normal import.

use super::tui_pty_support::{TuiPty, test_lock};

/// Verifies mixed rich-project entities appear in the real native semantic explorer.
#[test]
fn tui_rich_project_imports_and_lists_rich_artifacts() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_rich_golden();
    assert!(
        tui.read_until_for("Conversation", std::time::Duration::from_secs(60)),
        "{}",
        tui.diagnostic()
    );
    tui.send_slow(b"/explorer\r");
    assert!(tui.read_until("Semantic explorer"), "{}", tui.diagnostic());
    assert!(tui.read_until("budget.xlsx"), "{}", tui.diagnostic());
    assert!(tui.read_until("drawing.dxf"), "{}", tui.diagnostic());
    let screen = tui.screen();
    for evidence in ["budget.xlsx", "drawing.dxf"] {
        assert!(
            screen.contains(evidence),
            "missing {evidence}: {}",
            tui.diagnostic()
        );
    }
    assert!(tui.shutdown());
}
