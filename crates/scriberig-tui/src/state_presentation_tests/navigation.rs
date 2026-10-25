//! Covers read-only cursor navigation over the bounded semantic read models.

use crate::action::{Action, Direction};
use crate::effect::Effect;
use crate::inspector::LoadState;
use crate::selection::{PrimaryView, SemanticRowIdentity};
use crate::state::TuiState;
use crate::state::reduce;
use crate::view::diff::{DiffModel, DiffRow};
use crate::view::history::{HistoryModel, HistoryRow};
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Verifies diff and history cursor movement changes presentation state only.
///
/// A row move in either view must not request another diff, history, or session
/// read: both views are read-only presentations of an already loaded result.
#[test]
fn diff_and_history_navigation_stay_read_only() {
    let left = SnapshotId::new([40; 32]);
    let right = SnapshotId::new([41; 32]);
    let first = EntityId::from_bytes([42; 16]);
    let second = EntityId::from_bytes([43; 16]);
    let before = RevisionId::new([44; 32]);
    let after = RevisionId::new([45; 32]);
    let snapshot = SnapshotId::new([46; 32]);
    let mut state = TuiState {
        primary_view: PrimaryView::Diff,
        diff: LoadState::Ready(DiffModel {
            left,
            right,
            rows: vec![
                DiffRow {
                    entity: first,
                    before: Some(before),
                    after: Some(after),
                },
                DiffRow {
                    entity: second,
                    before: Some(before),
                    after: None,
                },
            ],
        }),
        history: LoadState::Ready(HistoryModel {
            snapshot,
            rows: vec![
                HistoryRow {
                    id: "migration-1".to_owned(),
                    base: left,
                    result: snapshot,
                    entities: vec![(first, Some(before))],
                    workspace: None,
                    source_change_id: None,
                },
                HistoryRow {
                    id: "migration-2".to_owned(),
                    base: right,
                    result: snapshot,
                    entities: Vec::new(),
                    workspace: None,
                    source_change_id: None,
                },
            ],
        }),
        cursor: 0,
        ..TuiState::default()
    };
    let diff_before = state.diff.clone();
    let history_before = state.history.clone();

    let effects = reduce(&mut state, Action::CursorMoved(Direction::Next));

    assert_eq!(effects, vec![Effect::None], "a diff row move must not act");
    assert_eq!(state.cursor, 1);
    assert_eq!(
        state.semantic_row_cursor,
        Some(SemanticRowIdentity::Diff(second, left, right)),
        "the moved diff row keeps its exact base/target pair"
    );
    assert_eq!(state.diff, diff_before, "the loaded diff stays untouched");

    state.primary_view = PrimaryView::History;
    state.cursor = 0;
    state.semantic_row_cursor = None;
    let effects = reduce(&mut state, Action::CursorMoved(Direction::Previous));

    assert_eq!(
        effects,
        vec![Effect::None],
        "a history row move must not act"
    );
    assert_eq!(state.cursor, 0);
    assert_eq!(
        state.history, history_before,
        "the loaded history stays untouched"
    );
}
