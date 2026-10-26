//! Verifies bounded upstream rendering and width behavior for semantic trees.

use super::super::common::MAX_ROWS;
use super::super::projection::{TreeProjection, sync_tree};
use super::test_support::{conversation_session, conversation_state, session_row, session_state};
use super::*;
use crate::view::conversation::ConversationModel;
use crate::view::session::SessionRowKind;
use rpi_tui::Component;
use scriberig_core::EntityId;

/// Columns an upstream tree row spends on its selection, indent, and expansion
/// markers before any projected label fits.
///
/// `rpi_tui::TreeList::render` clips the label to the remaining width but never
/// clips this prefix, so exact line-width containment holds only from this width
/// up. Narrower widths are still row-bounded and keep the semantic selection,
/// which `large_session_tree_stays_bounded_and_selected_across_widths` asserts.
const MIN_LABEL_WIDTH: usize = 8;

/// Reports whether rendered lines fit one render width.
///
/// - `lines`: Rendered component lines.
/// - `width`: Requested render width.
///
/// # Returns
/// `true` when no line exceeds the requested visible width.
fn within_width(lines: &[String], width: usize) -> bool {
    lines
        .iter()
        .all(|line| rpi_tui::utils::visible_width(line) <= width)
}

/// Builds a session projection with far more rows than the upstream viewport.
///
/// # Returns
/// The projected session tree items, selection, and details.
/// # Panics
/// Panics when the session projection is unavailable.
fn oversized_session_projection() -> TreeProjection {
    let root = EntityId::from_bytes([50; 16]);
    let mut rows = vec![session_row(SessionRowKind::Session, root, 0)];
    rows.extend((0..500_usize).map(|index| {
        let mut bytes = [51_u8; 16];
        bytes[0] = u8::try_from(index % 256).expect("row byte fits one byte");
        bytes[1] = u8::try_from(index / 256).expect("row page fits one byte");
        session_row(SessionRowKind::Branch, EntityId::from_bytes(bytes), 1)
    }));
    let state = session_state(rows);
    projection(&state).expect("session tree projection")
}

/// Verifies a large session tree renders no more rows than the upstream
/// viewport allows and keeps its semantic selection while the width changes.
#[test]
fn large_session_tree_stays_bounded_and_selected_across_widths() {
    let projection = oversized_session_projection();
    assert_eq!(projection.items.len(), 501);

    let mut slot = None;
    sync_tree(&mut slot, &projection).expect("tree accepts the projection");
    let selected = slot
        .as_ref()
        .expect("retained tree")
        .tree
        .selected_id()
        .expect("the first projected row is selected");

    for width in [1_usize, 5, MIN_LABEL_WIDTH, 20, 120] {
        let rendered = slot.as_ref().expect("retained tree").tree.render(width);
        assert!(!rendered.is_empty(), "width {width} rendered no rows");
        assert!(
            rendered.len() <= MAX_ROWS && rendered.len() < projection.items.len(),
            "width {width} rendered {} of {} projected rows",
            rendered.len(),
            projection.items.len()
        );
        assert_eq!(
            slot.as_ref()
                .expect("retained tree")
                .tree
                .selected_id()
                .as_deref(),
            Some(selected.as_str()),
            "width {width} changed the semantic selection"
        );
    }
}

/// Verifies upstream clips projected labels so each row fits the requested
/// width once the row's structural prefix fits that width.
#[test]
fn projected_labels_are_clipped_to_the_requested_width() {
    let projection = oversized_session_projection();
    let mut slot = None;
    sync_tree(&mut slot, &projection).expect("tree accepts the projection");

    for width in [MIN_LABEL_WIDTH, 20, 60, 120] {
        let rendered = slot.as_ref().expect("retained tree").tree.render(width);
        assert!(
            within_width(&rendered, width),
            "width {width} overflowed: {rendered:?}"
        );
    }
}

/// Verifies upstream width handling clips wide and combining Unicode safely.
#[test]
fn unicode_tree_labels_follow_upstream_display_width() {
    let mut row = session_row(SessionRowKind::Branch, EntityId::from_bytes([59; 16]), 0);
    row.label = "界e\u{301} wide label".to_owned();
    let state = session_state(vec![row]);
    let projection = projection(&state).expect("Unicode session projection");
    let mut slot = None;
    sync_tree(&mut slot, &projection).expect("upstream tree accepts Unicode label");

    for width in [MIN_LABEL_WIDTH, 12, 20] {
        let rendered = slot.as_ref().expect("retained tree").tree.render(width);
        assert!(
            within_width(&rendered, width),
            "width {width} overflowed for wide/combining text: {rendered:?}"
        );
    }
}

/// Renders streamed output through upstream `TreeList` without mutating session state.
#[test]
fn streamed_output_is_a_transient_upstream_tree_row() {
    let session_id = EntityId::from_bytes([77; 16]);
    let branch_id = EntityId::from_bytes([78; 16]);
    let session = conversation_session(session_id, branch_id, &[]);
    let operation =
        scriberig_core::OperationId::new("streamed-output-row").expect("valid operation identity");
    let mut conversation = ConversationModel::from_session(&session);
    conversation.insert_transient_output(branch_id, &operation, "live delta");
    let state = TuiState {
        primary_view: crate::selection::PrimaryView::Conversation,
        conversation: Some(conversation),
        session_snapshot: Some(session.clone()),
        ..TuiState::default()
    };

    let projection = projection(&state).expect("conversation projection");
    let tree = super::super::projection::tree_from_items(&projection.items)
        .expect("upstream tree accepts streamed rows");
    let rendered = tree.render(120).join("\n");

    assert!(rendered.contains("Assistant: live delta"), "{rendered}");
    assert!(
        state
            .session_snapshot
            .as_ref()
            .expect("authoritative session snapshot")
            .branches
            .iter()
            .find(|branch| branch.id == branch_id)
            .expect("owning branch")
            .turns
            .is_empty(),
        "the streamed presentation row must not become a durable turn"
    );
}

/// Builds one deep recursive session chain of the supplied length.
///
/// - `depths`: Number of nested session rows, each one level deeper than its
///   predecessor, modelling a recursively decomposed session.
///
/// # Returns
/// The projected session tree items for that chain.
/// # Panics
/// Panics when the session projection is unavailable.
fn deep_session_projection(depths: usize) -> TreeProjection {
    let mut rows = (0..depths)
        .map(|depth| {
            let mut bytes = [60_u8; 16];
            bytes[0] = u8::try_from(depth % 256).expect("row byte fits one byte");
            bytes[1] = u8::try_from(depth / 256).expect("row page fits one byte");
            session_row(SessionRowKind::Branch, EntityId::from_bytes(bytes), depth)
        })
        .collect::<Vec<_>>();
    // The first projected row must be a root, so the chain has one ancestor rail.
    rows[0].depth = 0;
    let state = session_state(rows);
    projection(&state).expect("deep session tree projection")
}

/// Verifies a deeply recursive session tree stays viewport-bounded and collapses
/// at its own structural parent row.
#[test]
fn deep_recursive_session_tree_stays_bounded_and_collapses_at_its_parent() {
    let projection = deep_session_projection(201);
    assert_eq!(projection.items.len(), 201);
    assert_eq!(
        projection
            .items
            .last()
            .expect("deepest projected row")
            .parent_id
            .as_deref(),
        Some(projection.items[projection.items.len() - 2].id.as_str()),
        "the deepest branch stays a child of its own structural parent row"
    );

    let mut slot = None;
    sync_tree(&mut slot, &projection).expect("tree accepts the deep projection");
    for width in [8_usize, 60, 120] {
        let rendered = slot.as_ref().expect("retained tree").tree.render(width);
        assert!(
            rendered.len() <= MAX_ROWS && rendered.len() < projection.items.len(),
            "depth 201 at width {width} rendered {} of {} projected rows",
            rendered.len(),
            projection.items.len()
        );
    }

    let mid = projection.items[100].id.clone();
    let child = projection.items[101].id.clone();
    let retained = slot.as_ref().expect("retained tree");
    assert!(
        retained
            .tree
            .visible_rows()
            .iter()
            .any(|row| row.id == child),
        "a deep chain starts fully expanded"
    );
    retained.tree.set_expanded(&mid, false);
    assert!(
        !retained
            .tree
            .visible_rows()
            .iter()
            .any(|row| row.id == child),
        "collapse must hide the whole deeper chain"
    );
}

/// Verifies a retained conversation reply anchor keeps its selected upstream row
/// while the render width changes.
#[test]
fn conversation_reply_anchor_stays_selected_across_width_changes() {
    let session = EntityId::from_bytes([70; 16]);
    let branch = EntityId::from_bytes([71; 16]);
    let anchor_turn = EntityId::from_bytes([72; 16]);
    let later_turn = EntityId::from_bytes([73; 16]);
    let value = conversation_session(session, branch, &[anchor_turn, later_turn]);
    let state = conversation_state(ConversationModel::from_session(&value), anchor_turn);
    let anchor_key = format!("activity:{anchor_turn}");

    let projection = projection(&state).expect("conversation tree projection");
    assert_eq!(projection.selected_id.as_deref(), Some(anchor_key.as_str()));

    let mut slot = None;
    sync_tree(&mut slot, &projection).expect("tree accepts the conversation projection");
    for width in [1_usize, 5, MIN_LABEL_WIDTH, 20, 120] {
        let rendered = slot.as_ref().expect("retained tree").tree.render(width);
        assert!(!rendered.is_empty(), "width {width} rendered no rows");
        assert!(
            rendered.len() <= MAX_ROWS,
            "width {width} overflowed the viewport"
        );
        assert_eq!(
            slot.as_ref()
                .expect("retained tree")
                .tree
                .selected_id()
                .as_deref(),
            Some(anchor_key.as_str()),
            "width {width} moved the reply anchor selection"
        );
        assert!(
            slot.as_ref()
                .expect("retained tree")
                .tree
                .visible_rows()
                .iter()
                .any(|row| row.id == anchor_key),
            "width {width} hid the reply anchor row"
        );
    }
}
