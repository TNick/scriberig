//! Verifies the native TUI semantic-agent golden path.

#[allow(dead_code)]
mod tui_pty_support;

use std::time::Duration;
use tui_pty_support::{TuiPty, test_lock};

/// TUI-GOLDEN-05 runs the deterministic agent through exact semantic scope.
#[test]
fn tui_golden_05_agent_uses_exact_semantic_scope_and_returns_compact_result() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_golden();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"Inspect the existing source structure.\r");
    assert!(
        tui.read_until("Use the shared code domain"),
        "{}",
        tui.diagnostic()
    );
    let session = tui.stored_session_id();
    let saved = tui.load_session(session);
    let tool_names: Vec<&str> = saved
        .branches
        .iter()
        .flat_map(|branch| branch.tool_invocations.iter())
        .map(|tool| tool.name.as_str())
        .collect();
    assert!(tool_names.contains(&"semantic_find"), "{tool_names:?}");
    assert!(tool_names.contains(&"semantic_render"), "{tool_names:?}");
    assert!(saved.branches.iter().any(|branch| {
        branch
            .result
            .as_ref()
            .and_then(|result| result.summary.as_deref())
            .is_some_and(|summary| summary.contains("shared code domain"))
    }));
    assert!(tui.source().contains("left + right"));
    tui_pty_support::onboarding_wait::wait_for_branch_completion(
        &tui,
        session,
        "agent root",
        Duration::from_secs(30),
    );
    assert!(tui.shutdown());
}
