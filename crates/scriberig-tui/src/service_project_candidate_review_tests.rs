//! Verifies durable semantic values become bounded TUI candidate review details.

use scriberig_core::EntityId;
use scriberig_project::SemanticCandidateChange;

use super::{append_materialization_receipt, review_semantic_value, semantic_change_details};

/// Shows typed before/after values through the shared TUI review projection.
#[test]
fn semantic_changes_render_as_readable_typed_values() {
    let details = semantic_change_details(&[
        SemanticCandidateChange {
            target: EntityId::new(),
            property: "cell.value".to_owned(),
            before: serde_json::json!({"Number": "10"}),
            after: serde_json::json!({"Number": "12"}),
        },
        SemanticCandidateChange {
            target: EntityId::new(),
            property: "table_cell.paragraph_text".to_owned(),
            before: serde_json::json!("parcel 124"),
            after: serde_json::json!("parcel 125"),
        },
    ]);

    assert_eq!(details.len(), 2);
    assert_eq!(
        details[0],
        "semantic change: cell.value Number \"10\" → Number \"12\""
    );
    assert_eq!(
        details[1],
        "semantic change: table_cell.paragraph_text Text \"parcel 124\" → Text \"parcel 125\""
    );
}

/// Truncates large semantic JSON values before adding them to TUI review state.
#[test]
fn semantic_value_preview_has_a_visible_bound() {
    let preview = review_semantic_value(&serde_json::json!("x".repeat(600)));

    assert!(preview.len() < 530);
    assert!(preview.ends_with("… truncated …"));
}

/// Shows aggregate and per-path recovery receipts after a failed apply.
#[test]
fn apply_failure_message_reports_durable_partial_materialization() {
    let message = append_materialization_receipt(
        "filesystem_write_failed",
        Some(scriberig_project::ProjectMaterializationReceipt {
            candidate: "candidate-1".to_owned(),
            state: "unknown".to_owned(),
            diagnostic: Some("recovery is required".to_owned()),
            paths: vec![
                scriberig_project::ProjectMaterializationPath {
                    path: "a.docx".to_owned(),
                    state: "applied".to_owned(),
                    diagnostic: None,
                },
                scriberig_project::ProjectMaterializationPath {
                    path: "b.docx".to_owned(),
                    state: "failed".to_owned(),
                    diagnostic: Some("permission denied".to_owned()),
                },
            ],
        }),
    );
    assert!(message.contains("materialization state: unknown"));
    assert!(message.contains("recovery is required"));
    assert!(message.contains("a.docx: applied"));
    assert!(message.contains("b.docx: failed (permission denied)"));
}
