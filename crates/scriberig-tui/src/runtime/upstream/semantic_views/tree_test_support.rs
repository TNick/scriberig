//! Shared row builders and projection assertions for the semantic tree tests.

use super::super::projection::TreeProjection;
use crate::inspector::LoadState;
use crate::state::TuiState;
use crate::view::session::{SessionModel, SessionRow, SessionRowKind};
use scriberig_core::EntityId;

/// Builds one bounded session tree row for projection assertions.
///
/// - `kind`: Public session object kind.
/// - `reference`: Stable semantic identity represented by the row.
/// - `depth`: Projected tree depth.
///
/// # Returns
/// A session row with a safe label.
pub(super) fn session_row(kind: SessionRowKind, reference: EntityId, depth: usize) -> SessionRow {
    SessionRow {
        reference,
        depth,
        label: format!("{kind:?} {reference}"),
        kind,
        candidate: None,
        snapshot: None,
        transient: false,
    }
}

/// Builds a ready Session view from the supplied semantic rows.
///
/// - `rows`: Ordered semantic session rows.
///
/// # Returns
/// Presentation state for the Session view.
pub(super) fn session_state(rows: Vec<SessionRow>) -> TuiState {
    let execution = scriberig_session::SessionExecutionSelection::default();
    TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: LoadState::Ready(SessionModel {
            rows,
            execution,
            tool_invocation_details: std::collections::BTreeMap::new(),
        }),
        ..TuiState::default()
    }
}

/// Replaces the Session view rows of an existing state.
///
/// - `state`: Presentation state receiving the authoritative rows.
/// - `rows`: New ordered semantic session rows.
pub(super) fn replace_session_rows(state: &mut TuiState, rows: Vec<SessionRow>) {
    if let LoadState::Ready(model) = &mut state.session {
        model.rows = rows;
    }
}

/// Reports whether every projected upstream item ID is distinct.
///
/// - `projection`: Active semantic tree projection.
///
/// # Returns
/// `true` when upstream tree item validation can accept the projection.
pub(super) fn ids_are_unique(projection: &TreeProjection) -> bool {
    let mut ids = projection
        .items
        .iter()
        .map(|item| &item.id)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids.len() == projection.items.len()
}
