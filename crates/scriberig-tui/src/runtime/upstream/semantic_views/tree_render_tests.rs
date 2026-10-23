//! Verifies bounded upstream rendering and width behavior for semantic trees.

use super::super::common::MAX_ROWS;
use super::super::projection::{TreeProjection, sync_tree};
use super::test_support::{session_row, session_state};
use super::*;
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
