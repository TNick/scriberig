//! Verifies safe model and tool detail projection bounds.

use scriberig_app::agent_console::detail::safe_tool_result_detail;

/// Bounds oversized result content and omits fields outside the safe allowlist.
#[test]
fn tool_result_detail_is_bounded_and_whitelisted() {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "summary": "x".repeat(8000),
        "private_payload": "FAKE_PRIVATE_RESULT_8391",
    }))
    .expect("result JSON");

    let (detail, truncated) = safe_tool_result_detail(&bytes);

    assert!(truncated);
    assert_eq!(detail["summary"].as_str().unwrap().chars().count(), 4096);
    assert!(detail.get("private_payload").is_none());
    assert!(detail.to_string().len() < 32_768);
}
