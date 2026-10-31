//! Tests semantic projection caching and retained upstream tree state.

use super::super::projection::{TreeProjection, sync_tree_for_view};
use super::ProjectionCache;
use crate::inspector::LoadState;
use crate::selection::PrimaryView;
use crate::state::TuiState;
use crate::view::session::{SessionModel, SessionRow, SessionRowKind};
use rpi_tui::TreeItem;
use scriberig_core::EntityId;
use std::collections::HashMap;

/// Creates one ready session tree for projection-cache tests.
///
/// # Returns
/// A TUI state with one stable semantic session row.
fn ready_session_state() -> TuiState {
    TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: LoadState::Ready(SessionModel {
            rows: vec![SessionRow {
                reference: EntityId::from_bytes([1; 16]),
                depth: 0,
                label: "Session row".to_owned(),
                kind: SessionRowKind::Session,
                candidate: None,
                snapshot: None,
                transient: false,
            }],
            execution: scriberig_session::SessionExecutionSelection::default(),
            tool_invocation_details: std::collections::BTreeMap::default(),
        }),
        ..TuiState::default()
    }
}

/// Reuses a semantic projection until an authoritative state revision changes.
#[test]
fn caches_tree_projection_between_render_revisions() {
    let mut cache = ProjectionCache::default();
    let state = ready_session_state();
    let first = cache.get(&state).expect("session projection");
    let repeated = cache.get(&state).expect("cached session projection");
    assert!(std::sync::Arc::ptr_eq(&first, &repeated));

    let mut changed = state;
    changed.render_revision = changed.render_revision.wrapping_add(1);
    let refreshed = cache.get(&changed).expect("refreshed session projection");
    assert!(!std::sync::Arc::ptr_eq(&first, &refreshed));
}

/// Advances the cache key whenever the state reducer accepts an action.
#[test]
fn state_reduction_advances_render_revision() {
    let mut state = TuiState::default();
    crate::state::reduce(&mut state, crate::action::Action::Noop);
    assert_eq!(state.render_revision, 1);
}

/// Reuses session rows and refreshes selection after cursor movement.
#[test]
fn session_cursor_movement_reuses_structure_and_refreshes_selection() {
    let mut state = ready_session_state();
    let selected = EntityId::from_bytes([2; 16]);
    let LoadState::Ready(model) = &mut state.session else {
        unreachable!();
    };
    model.rows.push(SessionRow {
        reference: selected,
        depth: 1,
        label: "Selected branch".to_owned(),
        kind: SessionRowKind::Branch,
        candidate: None,
        snapshot: None,
        transient: false,
    });
    let mut cache = ProjectionCache::default();
    let initial = cache.get(&state).expect("session projection");
    let rows = initial.items.as_ptr();
    drop(initial);
    crate::state::reduce(
        &mut state,
        crate::action::Action::CursorMoved(crate::action::Direction::Next),
    );
    let moved = cache.get(&state).expect("cached session projection");
    assert_eq!(
        moved.selected_id.as_deref(),
        Some(format!("session-row:Branch:{selected}").as_str())
    );
    assert_eq!(moved.items.len(), 2);
    assert!(moved.details.contains("Selected activity: Selected branch"));
    assert_eq!(
        moved.items.as_ptr(),
        rows,
        "cursor movement must retain the projected row allocation"
    );
}

/// Keeps upstream collapse state isolated when tree-backed views share row IDs.
#[test]
fn retained_tree_state_is_independent_per_semantic_view() {
    let rows = |label: &str| {
        vec![
            TreeItem {
                id: "plan-view:header:shared".to_owned(),
                label: label.to_owned(),
                parent_id: None,
            },
            TreeItem {
                id: "plan-view:node:shared:child".to_owned(),
                label: format!("{label} child"),
                parent_id: Some("plan-view:header:shared".to_owned()),
            },
        ]
    };
    let conversation = TreeProjection {
        items: rows("conversation"),
        selected_id: Some("plan-view:header:shared".to_owned()),
        details: String::new(),
    };
    let plan = TreeProjection {
        items: rows("plan"),
        selected_id: Some("plan-view:node:shared:child".to_owned()),
        details: String::new(),
    };
    let mut trees = HashMap::new();
    sync_tree_for_view(&mut trees, PrimaryView::Conversation, &conversation)
        .expect("conversation tree accepts its projection");
    let conversation_tree = trees
        .get(&PrimaryView::Conversation)
        .and_then(Option::as_ref)
        .expect("retained conversation tree");
    assert!(
        conversation_tree
            .tree
            .set_expanded("plan-view:header:shared", false)
    );

    sync_tree_for_view(&mut trees, PrimaryView::Plan, &plan)
        .expect("plan tree accepts its projection");
    let plan_tree = trees
        .get(&PrimaryView::Plan)
        .and_then(Option::as_ref)
        .expect("retained plan tree");
    assert_eq!(
        plan_tree.tree.visible_rows()[1].label,
        "plan child",
        "the Plans view keeps its independent expanded tree"
    );

    let conversation_tree = trees
        .get(&PrimaryView::Conversation)
        .and_then(Option::as_ref)
        .expect("conversation tree remains retained");
    assert_eq!(
        conversation_tree.tree.visible_rows().len(),
        1,
        "switching views must not overwrite conversation collapse state"
    );
    assert_eq!(conversation_tree.tree.items()[0].label, "conversation");
}
