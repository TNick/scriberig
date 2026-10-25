//! Verifies flat semantic rows keep exact semantic references through upstream lists.

use super::{list_from_state, reconcile_cursor, rows_for_state};
use crate::explorer::{ExplorerModel, ExplorerRow};
use crate::inspector::LoadState;
use crate::selection::{EntitySelection, PrimaryView, SemanticRowIdentity};
use crate::state::TuiState;
use crate::view::diff::{DiffModel, DiffRow};
use crate::view::operations::OperationModel;
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Builds an Explorer presentation state over the supplied semantic rows.
///
/// - `rows`: Ordered semantic explorer rows.
///
/// # Returns
/// Presentation state whose active view is the semantic explorer.
fn explorer_state(rows: Vec<ExplorerRow>) -> TuiState {
    TuiState {
        primary_view: PrimaryView::Explorer,
        explorer: ExplorerModel {
            root: None,
            rows,
            next: None,
        },
        ..TuiState::default()
    }
}

/// Builds one explorer row for an exact entity/revision/snapshot selection.
///
/// - `entity`: Conceptual entity identity.
/// - `revision`: Exact revision, when the row resolved one.
/// - `snapshot`: Snapshot context, when the row carries one.
/// - `label`: Display text that must never become part of the row identity.
///
/// # Returns
/// An inspectable explorer row.
fn explorer_row(
    entity: EntityId,
    revision: Option<RevisionId>,
    snapshot: Option<SnapshotId>,
    label: &str,
) -> ExplorerRow {
    ExplorerRow {
        selection: EntitySelection {
            entity,
            revision,
            snapshot,
        },
        label: label.to_owned(),
        kind: Some("rust function".to_owned()),
        inspectable: true,
    }
}

/// Verifies each explorer row carries the exact semantic selection it activates.
#[test]
fn explorer_rows_store_exact_entity_revision_and_snapshot_references() {
    let entity = EntityId::from_bytes([11; 16]);
    let revision = RevisionId::new([12; 32]);
    let snapshot = SnapshotId::new([13; 32]);
    let state = explorer_state(vec![explorer_row(
        entity,
        Some(revision),
        Some(snapshot),
        "compute_total",
    )]);

    let rows = rows_for_state(&state).expect("explorer rows");

    assert_eq!(
        rows[0].identity,
        SemanticRowIdentity::Explorer {
            entity,
            revision: Some(revision),
            snapshot: Some(snapshot),
            root: false,
        }
    );
    assert_eq!(
        rows[0].id,
        format!("explorer:entity:{entity}:{revision}:{snapshot}")
    );
}

/// Verifies revision and snapshot identities stay separate row components.
///
/// The same entity projected at a revision, at a snapshot without a revision,
/// and without either must produce three distinct semantic rows, so a snapshot
/// context can never be read as a revision identity.
#[test]
fn revision_and_snapshot_contexts_stay_distinguishable_for_one_entity() {
    let entity = EntityId::from_bytes([21; 16]);
    let revision = RevisionId::new([22; 32]);
    let snapshot = SnapshotId::new([23; 32]);
    let state = explorer_state(vec![
        explorer_row(entity, Some(revision), Some(snapshot), "at revision"),
        explorer_row(entity, None, Some(snapshot), "at snapshot"),
        explorer_row(entity, None, None, "conceptual"),
    ]);

    let rows = rows_for_state(&state).expect("explorer rows");

    let ids = rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            format!("explorer:entity:{entity}:{revision}:{snapshot}"),
            format!("explorer:entity:{entity}:none:{snapshot}"),
            format!("explorer:entity:{entity}:none:none"),
        ]
    );
    assert_eq!(
        rows.iter()
            .map(|row| match &row.identity {
                SemanticRowIdentity::Explorer {
                    revision, snapshot, ..
                } => (*revision, *snapshot),
                other => panic!("explorer rows carry an explorer identity, got {other:?}"),
            })
            .collect::<Vec<_>>(),
        vec![
            (Some(revision), Some(snapshot)),
            (None, Some(snapshot)),
            (None, None),
        ]
    );
}

/// Verifies upstream row keys are derived from semantic references only.
///
/// A hostile display label that looks like a source path must reach the upstream
/// row as sanitized text; the selectable key stays the semantic reference.
#[test]
fn semantic_row_keys_never_use_display_text_or_source_paths() {
    let entity = EntityId::from_bytes([31; 16]);
    let snapshot = SnapshotId::new([32; 32]);
    let state = explorer_state(vec![explorer_row(
        entity,
        None,
        Some(snapshot),
        "src/main.rs\u{1b}[31m hidden\u{7}",
    )]);

    let (list, rows) = list_from_state(&state).expect("explorer list");
    let item = list.get_selected_item().expect("selected upstream row");

    assert_eq!(item.value, rows[0].id);
    assert_eq!(
        item.value,
        format!("explorer:entity:{entity}:none:{snapshot}")
    );
    assert!(
        !item.value.contains('/') && !item.value.contains("main"),
        "row key must not carry display text: {}",
        item.value
    );
    assert!(
        !item.label.contains('\u{1b}') && !item.label.contains('\u{7}'),
        "upstream label must stay terminal-safe: {:?}",
        item.label
    );
    assert!(
        item.search_key().contains(&entity.to_string()),
        "the upstream filter must match the semantic reference: {:?}",
        item.search_key()
    );
}

/// Verifies upstream selection resolves by semantic identity, not row position.
#[test]
fn upstream_selection_follows_the_retained_semantic_identity() {
    let first = EntityId::from_bytes([41; 16]);
    let second = EntityId::from_bytes([42; 16]);
    let snapshot = SnapshotId::new([43; 32]);
    let rows = vec![
        explorer_row(first, None, Some(snapshot), "first"),
        explorer_row(second, None, Some(snapshot), "second"),
    ];
    let mut state = explorer_state(rows);
    state.cursor = 0;
    state.semantic_row_cursor = Some(SemanticRowIdentity::Explorer {
        entity: second,
        revision: None,
        snapshot: Some(snapshot),
        root: false,
    });

    let (list, projected) = list_from_state(&state).expect("explorer list");
    let selected = list.get_selected_item().expect("selected upstream row");

    assert_eq!(selected.value, projected[1].id);
    assert_eq!(
        projected[1].identity,
        SemanticRowIdentity::Explorer {
            entity: second,
            revision: None,
            snapshot: Some(snapshot),
            root: false,
        }
    );
}

/// Verifies an authoritative reorder keeps the semantic row cursor.
#[test]
fn reconcile_cursor_follows_the_semantic_identity_after_a_reorder() {
    let first = EntityId::from_bytes([51; 16]);
    let second = EntityId::from_bytes([52; 16]);
    let snapshot = SnapshotId::new([53; 32]);
    let mut state = explorer_state(vec![
        explorer_row(first, None, Some(snapshot), "first"),
        explorer_row(second, None, Some(snapshot), "second"),
    ]);
    let retained = SemanticRowIdentity::Explorer {
        entity: second,
        revision: None,
        snapshot: Some(snapshot),
        root: false,
    };
    state.semantic_row_cursor = Some(retained.clone());
    state.cursor = 1;

    state.explorer.rows.reverse();
    reconcile_cursor(&mut state);

    assert_eq!(
        state.cursor, 0,
        "the identity moved, not the numeric cursor"
    );
    assert_eq!(state.semantic_row_cursor.as_ref(), Some(&retained));
}

/// Verifies diff rows bind each changed entity to the exact base/target pair.
#[test]
fn diff_rows_keep_the_exact_base_and_target_snapshot_pair() {
    let left = SnapshotId::new([61; 32]);
    let right = SnapshotId::new([62; 32]);
    let entity = EntityId::from_bytes([63; 16]);
    let before = RevisionId::new([64; 32]);
    let after = RevisionId::new([65; 32]);
    let state = TuiState {
        primary_view: PrimaryView::Diff,
        diff: LoadState::Ready(DiffModel {
            left,
            right,
            rows: vec![DiffRow {
                entity,
                before: Some(before),
                after: Some(after),
            }],
        }),
        ..TuiState::default()
    };

    let rows = rows_for_state(&state).expect("diff rows");

    assert_eq!(
        rows[0].identity,
        SemanticRowIdentity::Diff(entity, left, right)
    );
    assert_eq!(rows[0].id, format!("diff:{entity}:{left}:{right}"));
    assert_eq!(rows[0].description, format!("{before} → {after}"));
}

/// Verifies operation rows project the shared lifecycle without adding state.
#[test]
fn operation_rows_carry_the_shared_operation_identity_and_status() {
    let running = scriberig_app::OperationRecord::queued(
        scriberig_core::OperationId::new("running").expect("operation id"),
        scriberig_app::OperationKind::Query,
    );
    let mut completed = scriberig_app::OperationRecord::queued(
        scriberig_core::OperationId::new("completed").expect("operation id"),
        scriberig_app::OperationKind::History,
    );
    completed.status = scriberig_app::OperationStatus::Completed;
    completed.result_handles = vec!["snapshot:test".to_owned()];

    let state = TuiState {
        primary_view: PrimaryView::Operations,
        operations: vec![
            OperationModel::from_record(&running),
            OperationModel::from_record(&completed),
        ],
        ..TuiState::default()
    };

    let rows = rows_for_state(&state).expect("operation rows");

    assert_eq!(
        rows.iter()
            .map(|row| row.identity.clone())
            .collect::<Vec<_>>(),
        vec![
            SemanticRowIdentity::Operation(
                scriberig_core::OperationId::new("running").expect("id")
            ),
            SemanticRowIdentity::Operation(
                scriberig_core::OperationId::new("completed").expect("id")
            ),
        ]
    );
    assert_eq!(rows[0].id, "operation:op_running");
    assert_eq!(rows[1].id, "operation:op_completed");
    assert!(
        rows[1].label.contains("Completed") && rows[1].label.contains("History"),
        "the shared lifecycle values must reach the visible row: {:?}",
        rows[1].label
    );
    assert_eq!(rows[1].description, "op_completed");
}
