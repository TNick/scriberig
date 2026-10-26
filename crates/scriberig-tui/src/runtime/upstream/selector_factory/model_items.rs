//! Projects safe model metadata into searchable upstream selector rows.

use rpi_tui::SelectItem;

use super::readiness::{model_status, safe_selector_text};

/// Projects searchable upstream model rows for provider/model pickers.
///
/// - `library`: Credential-safe rows from the shared model library.
/// - `provider_filter`: Optional provider identity limiting the result.
///
/// # Returns
/// Upstream selector rows with stable values and searchable display metadata.
pub(super) fn model_selector_items(
    library: &scriberig_protocol::ModelLibraryResponse,
    provider_filter: Option<&str>,
) -> Vec<SelectItem> {
    library
        .models
        .iter()
        .filter(|model| provider_filter.is_none_or(|provider| model.provider_id == provider))
        .map(|model| {
            let label = safe_selector_text(&model.display_name);
            let status = model_status(library, &model.provider_id, &model.provider_model_id);
            let provider_model = format!(
                "{}/{} · {status}",
                safe_selector_text(&model.provider_id),
                safe_selector_text(&model.provider_model_id)
            );
            let search = format!(
                "{} {} {}",
                safe_selector_text(&model.display_name),
                safe_selector_text(&model.provider_id),
                safe_selector_text(&model.provider_model_id)
            );
            SelectItem::new(&model.id, &label)
                .with_description(&provider_model)
                .with_search_text(&search)
        })
        .collect()
}
