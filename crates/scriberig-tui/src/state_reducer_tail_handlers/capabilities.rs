//! Handles capability and operation list refreshes.

use super::*;

/// Applies refreshed runtime capabilities to the shell state.
///
/// - `state`: TUI presentation receiving runtime capabilities.
/// - `capabilities`: Public capability descriptors from the application.
///
/// A no-op effect after updating connection state.
/// # Returns
/// The operation result.
pub fn capabilities_loaded(
    state: &mut TuiState,
    capabilities: Vec<scriberig_app::RuntimeCapability>,
) -> Vec<Effect> {
    state.capabilities = capabilities;
    state.connection = ConnectionState::Connected;
    state.stale = false;
    state.shell_message = "Semantic service attached.".to_owned();
    vec![Effect::ReadPresentationPreferences]
}

/// Updates the local presentation cache from an authoritative operation snapshot.
///
/// - `state`: TUI presentation receiving operation records.
/// - `records`: Current records supplied by the shared application service.
///
/// A no-op effect after refreshing operation models.
/// # Returns
/// The operation result.
pub fn operations_loaded(
    state: &mut TuiState,
    records: Vec<scriberig_app::OperationRecord>,
) -> Vec<Effect> {
    for record in records {
        let model = crate::view::operations::OperationModel::from_record(&record);
        if let Some(existing) = state
            .operations
            .iter_mut()
            .find(|existing| existing.id == model.id)
        {
            *existing = model;
        } else {
            state.operations.push(model);
        }
    }
    if state.primary_view == crate::selection::PrimaryView::Operations {
        if state.semantic_row_cursor.is_none()
            && let Some(index) = state
                .operations
                .iter()
                .rposition(crate::view::operations::OperationModel::cancellable)
        {
            state.cursor = index;
        }
        crate::runtime::reconcile_semantic_row_cursor(state);
    }
    vec![Effect::None]
}
