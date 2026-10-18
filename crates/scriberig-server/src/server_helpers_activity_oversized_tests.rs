//! Verifies oversized tool arguments stay out of the activity page.

use super::tool_presentation::bounded_tool_arguments;

/// Omits oversized structured parameters from the primary activity page.
#[test]
fn oversized_tool_arguments_are_not_projected_inline() {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "query": "q".repeat(5000),
    }))
    .expect("argument JSON");

    assert!(bounded_tool_arguments(&bytes).is_none());
}
