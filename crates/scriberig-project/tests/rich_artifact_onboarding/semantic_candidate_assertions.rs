//! Asserts exact semantic before/after data on durable rich-artifact candidates.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_spreadsheet::{CellId, CellValue, Formula};
use scriberig_project::{ProjectCandidate, ProjectService};

/// Proposes the fixture formula edit used by the mixed-project verification.
///
/// # Parameters
/// - `service`: Project service for candidate creation.
/// - `project`: Project owning the workbook.
/// - `snapshot`: Exact imported snapshot.
/// - `workbook`: Stable semantic workbook identity.
/// - `worksheet`: Stable semantic worksheet identity.
///
/// # Returns
/// Durable candidate for the fixture's B3 formula change.
///
/// # Panics
/// Panics when candidate creation fails.
pub(super) fn propose_formula_cell_edit(
    service: &ProjectService,
    project: ProjectId,
    snapshot: SnapshotId,
    workbook: EntityId,
    worksheet: EntityId,
) -> ProjectCandidate {
    let target = CellId::new(workbook, worksheet, 3, 2).entity();
    service
        .propose_xlsx_semantic_cell_edit(
            project,
            snapshot,
            target,
            &CellValue::Formula(Formula {
                expression: "=A3*3".into(),
                cached_value: None,
            }),
        )
        .expect("mixed-project spreadsheet candidate")
}

/// Verifies and reloads the typed value diff for one spreadsheet formula edit.
///
/// # Parameters
/// - `service`: Project service that persisted the candidate.
/// - `candidate`: Newly proposed exact-base candidate.
///
/// # Panics
/// Panics when target, before/after values, or durable reload differ from expectations.
pub(super) fn assert_formula_cell_diff(service: &ProjectService, candidate: &ProjectCandidate) {
    let snapshot = candidate.base_snapshot;
    let target = candidate.semantic_target.expect("cell target");
    super::fixture_evidence::assert_candidate_target(candidate, snapshot, target);
    assert_persisted_semantic_change(
        service,
        candidate,
        target,
        "cell.value",
        &serde_json::json!({
            "Formula": {
                "expression": "=A3*2",
                "cached_value": {"Number": "20"}
            }
        }),
        &serde_json::json!({"Formula": {"expression": "=A3*3", "cached_value": null}}),
    );
}

/// Asserts and reloads one exact semantic before/after change.
///
/// # Parameters
/// - `service`: Project service that persisted the candidate.
/// - `candidate`: Durable candidate under review.
/// - `target`: Exact semantic entity changed.
/// - `property`: Stable domain property key.
/// - `before`: Expected prior semantic value.
/// - `after`: Expected proposed semantic value.
///
/// # Panics
/// Panics when the change differs or fails to survive candidate reload.
pub(super) fn assert_persisted_semantic_change(
    service: &ProjectService,
    candidate: &ProjectCandidate,
    target: EntityId,
    property: &str,
    before: &serde_json::Value,
    after: &serde_json::Value,
) {
    assert_eq!(candidate.semantic_changes.len(), 1);
    let change = &candidate.semantic_changes[0];
    assert_eq!(change.target, target);
    assert_eq!(change.property, property);
    assert_eq!(&change.before, before);
    assert_eq!(&change.after, after);
    let reloaded = service
        .candidate(&candidate.id)
        .expect("reload candidate")
        .expect("durable candidate");
    assert_eq!(reloaded.semantic_changes, candidate.semantic_changes);
}

/// Verifies and reloads the typed text diff for one DOCX paragraph edit.
///
/// # Parameters
/// - `service`: Project service that persisted the candidate.
/// - `candidate`: Newly proposed exact-base candidate.
/// - `snapshot`: Exact source snapshot for the proposal.
/// - `target`: Stable semantic paragraph identity selected by the edit.
///
/// # Panics
/// Panics when target, before/after values, or durable reload differ from expectations.
pub(super) fn assert_paragraph_diff(
    service: &ProjectService,
    candidate: &ProjectCandidate,
    snapshot: scriberig_core::SnapshotId,
    target: EntityId,
) {
    super::fixture_evidence::assert_candidate_target(candidate, snapshot, target);
    assert_persisted_semantic_change(
        service,
        candidate,
        target,
        "paragraph.text",
        &serde_json::json!("Existing site area"),
        &serde_json::json!("Proposed site area"),
    );
}
