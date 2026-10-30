//! Tests semantic candidate, history, diff, and projection view rendering.

use super::{changes, diff, history, overview, projection, spreadsheet};
use crate::inspector::LoadState;
use crate::state::TuiState;
use crate::view::ArtifactMembersModel;
use crate::view::artifacts::{ArtifactMemberRow, ArtifactPageModel, ArtifactRow};
use crate::view::changes::{ChangeReviewModel, ReviewValidation};
use crate::view::diff::{DiffModel, DiffRow};
use crate::view::history::{HistoryModel, HistoryRow};
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Shows artifact fidelity restrictions and exact member evidence in the terminal view.
#[test]
fn overview_renders_bounded_artifact_details_and_read_only_reason() {
    let state = TuiState {
        artifact_page: LoadState::Ready(ArtifactPageModel {
            project: "prj_010101".to_owned(),
            artifacts: vec![ArtifactRow {
                id: "ent_artifact".to_owned(),
                logical_key: "roads".to_owned(),
                kind: "shapefile".to_owned(),
                adapter: "shapefile v1".to_owned(),
                scan_id: "aabb".repeat(16),
                baseline_digest: "1122".repeat(16),
                observed_digest: Some("3344".repeat(16)),
                synchronization_status: "modified".to_owned(),
                fidelity: "read-only-derived".to_owned(),
                opaque_preservation: "unproven".to_owned(),
                diagnostic: Some("writer is unavailable".to_owned()),
                members: vec![crate::view::artifacts::ArtifactMemberRow {
                    key: "geometry".to_owned(),
                    project_path: Some("roads.shp".to_owned()),
                    container_part: None,
                    digest: "ccdd".repeat(16),
                }],
                member_count: 21,
                references: vec!["project-path: roads.prj".to_owned()],
            }],
            total: 1,
            next_offset: None,
        }),
        artifact_members: LoadState::Ready(ArtifactMembersModel {
            project: "prj_010101".to_owned(),
            artifact: "ent_artifact".to_owned(),
            members: vec![ArtifactMemberRow {
                key: "container:word/document.xml".to_owned(),
                project_path: None,
                container_part: Some("word/document.xml".to_owned()),
                digest: "ddee".repeat(16),
            }],
            total: 21,
            next_offset: Some(1),
        }),
        ..TuiState::default()
    };
    let rendered = overview(&state);
    assert!(rendered.contains("EDITING RESTRICTED"));
    assert!(rendered.contains("writer is unavailable"));
    assert!(rendered.contains("SOURCE OUT OF SYNC"));
    assert!(rendered.contains("baseline 1122"));
    assert!(rendered.contains("observed 3344"));
    assert!(rendered.contains("roads.shp"));
    assert!(rendered.contains("project-path: roads.prj"));
    assert!(rendered.contains("/artifact-members prj_010101 ent_artifact 0"));
    assert!(rendered.contains("word/document.xml"));
    assert!(rendered.contains("/artifact-members prj_010101 ent_artifact 1"));
}

/// Shows exact cell identity, authored formula, and cached result as separate facts.
#[test]
fn overview_renders_bounded_spreadsheet_values_and_fidelity_reason() {
    let state = TuiState {
        spreadsheet_page: LoadState::Ready(scriberig_protocol::SpreadsheetCellsResponse {
            project: "project-1".to_owned(),
            snapshot: "snapshot-1".to_owned(),
            worksheet: "worksheet-1".to_owned(),
            artifact_capability: Some(scriberig_protocol::ArtifactEditCapability::ReadOnlyDerived),
            artifact_diagnostic: Some("unsupported formula preserved read-only".to_owned()),
            cells: vec![scriberig_protocol::SpreadsheetCellView {
                id: "cell-7".to_owned(),
                row: 3,
                column: 2,
                value: scriberig_protocol::SpreadsheetCellValue::Formula {
                    expression: "=A3*2".to_owned(),
                    cached_value: Some(Box::new(scriberig_protocol::SpreadsheetCellValue::Number(
                        "20".to_owned(),
                    ))),
                },
                style: None,
            }],
            total: 1,
            has_more: false,
        }),
        ..TuiState::default()
    };

    let output = spreadsheet::render(&state.spreadsheet_page);

    assert!(output.contains("Worksheet: worksheet-1"));
    assert!(output.contains("EDITING RESTRICTED"));
    assert!(output.contains("B3 · cell-7"));
    assert!(output.contains("Formula: =A3\\*2"));
    assert!(output.contains("Cached result: 20"));
}

/// Renders an exact selected cell formula separately from its cached result.
#[test]
fn overview_renders_exact_spreadsheet_cell_inspection() {
    let state = TuiState {
        spreadsheet_cell: LoadState::Ready(scriberig_protocol::SpreadsheetCellInspectResponse {
            project: "project-2".to_owned(),
            snapshot: "snapshot-2".to_owned(),
            worksheet: "worksheet-2".to_owned(),
            row: 12,
            column: 28,
            cell: Some(scriberig_protocol::SpreadsheetCellView {
                id: "cell-28".to_owned(),
                row: 12,
                column: 28,
                value: scriberig_protocol::SpreadsheetCellValue::Formula {
                    expression: "=A12*2".to_owned(),
                    cached_value: Some(Box::new(scriberig_protocol::SpreadsheetCellValue::Number(
                        "24".to_owned(),
                    ))),
                },
                style: None,
            }),
        }),
        ..TuiState::default()
    };

    let output = spreadsheet::render_cell(&state.spreadsheet_cell);

    assert!(output.contains("Coordinate: AB12"));
    assert!(output.contains("Semantic cell: cell-28"));
    assert!(output.contains("Formula: =A12\\*2"));
    assert!(output.contains("Cached result: 24"));
}

/// Keeps exact candidate identity and evidence visible in review Markdown.
#[test]
fn changes_projection_shows_snapshots_validation_and_revision_evidence() {
    let base = SnapshotId::new([1; 32]);
    let result = SnapshotId::new([2; 32]);
    let entity = EntityId::from_bytes([3; 16]);
    let before = RevisionId::new([4; 32]);
    let after = RevisionId::new([5; 32]);
    let review = ChangeReviewModel {
        base_snapshot: base,
        candidate_snapshot: Some(result),
        change: "candidate-7".to_owned(),
        branch: None,
        affected_entities: vec![entity],
        deltas: vec![
            sr_change::EntityDelta::new(entity, Some(before), Some(after)).expect("revision delta"),
        ],
        details: vec!["materialized semantic projection".to_owned()],
        conflicts: Vec::new(),
        validation: ReviewValidation::Passed,
        validation_profile: Some("strict-policy".to_owned()),
        requires_confirmation: true,
    };
    let state = TuiState {
        review: LoadState::Ready(review),
        ..TuiState::default()
    };

    let output = changes(&state);

    assert!(output.contains(&format!("Base snapshot: {base}")));
    assert!(output.contains(&format!("Result snapshot: {result}")));
    assert!(output.contains("Validation profile: strict-policy"));
    assert!(output.contains(&format!("entity {entity}")));
    assert!(output.contains(&before.to_string()));
    assert!(output.contains(&after.to_string()));
    assert!(output.contains("materialized semantic projection"));
}

/// Explains why review cannot be confirmed without a result snapshot.
#[test]
fn changes_projection_blocks_confirmation_without_result_identity() {
    let state = TuiState {
        review: LoadState::Ready(ChangeReviewModel {
            base_snapshot: SnapshotId::new([6; 32]),
            candidate_snapshot: None,
            change: "candidate-no-result".to_owned(),
            branch: None,
            affected_entities: Vec::new(),
            deltas: Vec::new(),
            details: Vec::new(),
            conflicts: Vec::new(),
            validation: ReviewValidation::Passed,
            validation_profile: None,
            requires_confirmation: true,
        }),
        ..TuiState::default()
    };

    assert!(changes(&state).contains("Confirmation is unavailable"));
}

/// Keeps apply failures visible before long candidate diffs push them off-screen.
#[test]
fn changes_projection_shows_operation_error_before_candidate_content() {
    let state = TuiState {
        operation_error: Some((
            "filesystem_conflict".to_owned(),
            "source changed externally: Cargo.toml".to_owned(),
        )),
        review: LoadState::Ready(ChangeReviewModel {
            base_snapshot: SnapshotId::new([9; 32]),
            candidate_snapshot: Some(SnapshotId::new([10; 32])),
            change: "candidate-conflict".to_owned(),
            branch: None,
            affected_entities: Vec::new(),
            deltas: Vec::new(),
            details: vec!["candidate diff".to_owned()],
            conflicts: Vec::new(),
            validation: ReviewValidation::Passed,
            validation_profile: None,
            requires_confirmation: true,
        }),
        ..TuiState::default()
    };

    let output = changes(&state);
    let error_position = output.find("filesystem_conflict").expect("visible error");
    let detail_position = output.find("candidate diff").expect("candidate details");
    assert!(error_position < detail_position);
}

/// Retains exact projection identity without inventing synchronization state.
#[test]
fn projection_shows_exact_metadata_and_safe_content() {
    let entity = EntityId::from_bytes([7; 16]);
    let snapshot = SnapshotId::new([8; 32]);
    let state = TuiState {
        projection: LoadState::Ready(crate::view::projection::ProjectionModel {
            projection: Some("projection-1".to_owned()),
            source_entity: entity,
            source_path: Some("src/main.rs".to_owned()),
            source_revision: None,
            style: "markdown".to_owned(),
            renderer: "renderer-v1".to_owned(),
            content: "safe content".to_owned(),
            synchronization: None,
            base_snapshot: snapshot,
        }),
        ..TuiState::default()
    };

    let output = projection(&state);

    assert!(output.contains("projection-1"));
    assert!(output.contains(&entity.to_string()));
    assert!(output.contains("src/main.rs"));
    assert!(output.contains(&snapshot.to_string()));
    assert!(output.contains("safe content"));
    assert!(!output.contains("synchronized"));
}

/// Keeps both exact diff sides and every entity revision transition in the diff document.
#[test]
fn diff_document_names_both_exact_snapshots_and_each_entity_transition() {
    let base = SnapshotId::new([20; 32]);
    let target = SnapshotId::new([21; 32]);
    let changed = EntityId::from_bytes([22; 16]);
    let removed = EntityId::from_bytes([23; 16]);
    let before = RevisionId::new([24; 32]);
    let after = RevisionId::new([25; 32]);
    let state = TuiState {
        diff: LoadState::Ready(DiffModel {
            left: base,
            right: target,
            rows: vec![
                DiffRow {
                    entity: changed,
                    before: Some(before),
                    after: Some(after),
                },
                DiffRow {
                    entity: removed,
                    before: Some(before),
                    after: None,
                },
            ],
        }),
        ..TuiState::default()
    };

    let output = diff(&state);

    assert!(
        output.contains(&format!("Base snapshot: {base}")),
        "the exact base snapshot is missing: {output}"
    );
    assert!(
        output.contains(&format!("Target snapshot: {target}")),
        "the exact target snapshot is missing: {output}"
    );
    assert!(output.contains(&format!("{changed} · {before} → {after}")));
    assert!(output.contains(&format!("{removed} · {before} → none")));
}

/// Keeps the requested snapshot and each migration's exact identities in history.
#[test]
fn history_document_names_the_snapshot_and_each_migration_identity() {
    let snapshot = SnapshotId::new([30; 32]);
    let migration_base = SnapshotId::new([31; 32]);
    let entity = EntityId::from_bytes([32; 16]);
    let revision = RevisionId::new([33; 32]);
    let state = TuiState {
        history: LoadState::Ready(HistoryModel {
            snapshot,
            rows: vec![HistoryRow {
                id: "migration-7".to_owned(),
                base: migration_base,
                result: snapshot,
                entities: vec![(entity, Some(revision))],
                workspace: None,
                source_change_id: Some("candidate-7".to_owned()),
                semantic_operation: Some("geo.set_attribute".to_owned()),
            }],
        }),
        ..TuiState::default()
    };

    let output = history(&state);

    assert!(
        output.contains(&format!("Snapshot: {snapshot}")),
        "the requested snapshot identity is missing: {output}"
    );
    assert!(output.contains("Operation: geo.set_attribute"));
    assert!(output.contains("Candidate: candidate-7"));
    assert!(output.contains(&format!("migration-7 · {migration_base} → {snapshot}")));
    assert!(output.contains(&format!("{entity} · revision {revision}")));
}
