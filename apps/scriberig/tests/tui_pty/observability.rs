//! Exercises visible prompt and reasoning controls through the real TUI.

use super::tui_pty_support::TuiPty;

/// Verifies captured prompt and safe reasoning controls update the rendered TUI.
#[test]
fn tui_pty_toggles_captured_prompt_and_reasoning_visibility() {
    let _lock = super::tui_pty_support::test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    tui.send_slow(b"1+1\r");
    assert!(tui.read_until("You: 1+1"), "{}", tui.diagnostic());
    assert!(tui.read_until("BranchResult · 2"), "{}", tui.diagnostic());

    toggle_presentation_command(&mut tui, "prompt", "Tool 24 name");
    assert!(tui.settle(), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("Tool 24 name"),
        "shown request omitted its captured tools: {}",
        tui.diagnostic()
    );

    toggle_presentation_command(&mut tui, "prompt", "Captured model requests hidden");
    assert!(
        tui.read_until_not("Tool 24 name"),
        "hidden request details remained in the rendered TUI: {}",
        tui.diagnostic()
    );

    toggle_presentation_command(&mut tui, "reasoning", "Safe reasoning summaries shown");
    toggle_presentation_command(&mut tui, "reasoning", "Safe reasoning summaries hidden");
    assert!(tui.shutdown());
}

/// Toggles a presentation control through the visible command input.
///
/// # Parameters
/// - `tui`: Running pseudo-terminal TUI process.
/// - `control`: Control name accepted by the `/view` command.
/// - `expected`: Rendered confirmation or content expected after the command.
///
/// # Panics
/// Panics when the command result is not visible.
fn toggle_presentation_command(tui: &mut TuiPty, control: &str, expected: &str) {
    tui.send_slow(format!("/view {control}\r").as_bytes());
    assert!(tui.read_until(expected), "{}", tui.diagnostic());
}
