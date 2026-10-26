//! Verifies the upstream model picker remains searchable with large catalogs.

use std::sync::Arc;

use rpi_tui::{SearchableSelectList, SelectList};
use scriberig_protocol::{
    ModelLibraryResponse, PublicModelCostSnapshot, PublicModelDescriptor, PublicModelInputModality,
    PublicModelMetadataSnapshot, model_row_id,
};

use super::model_selector_items;

/// Keeps a model selectable after projecting a catalog with realistic breadth.
#[test]
fn searchable_model_selector_finds_a_late_catalog_row() {
    let models = (0..2_500)
        .map(|index| {
            let provider_model_id = format!("fixture-model-{index:04}");
            PublicModelDescriptor {
                id: model_row_id("fixture-provider", &provider_model_id),
                display_name: format!("Fixture Model {index:04}"),
                provider_id: "fixture-provider".to_owned(),
                provider_model_id,
                thinking_levels: Vec::new(),
                metadata_snapshot: PublicModelMetadataSnapshot {
                    schema_version: "fixture-v1".to_owned(),
                    api_id: "openai-completions".to_owned(),
                    reasoning: false,
                    input: vec![PublicModelInputModality::Text],
                    cost: PublicModelCostSnapshot {
                        input: 0.0,
                        output: 0.0,
                        cache_read: 0.0,
                        cache_write: 0.0,
                        tiers: Vec::new(),
                    },
                    cost_configured: false,
                    context_window: 32_000,
                    max_tokens: 4_000,
                },
            }
        })
        .collect();
    let library = ModelLibraryResponse {
        models,
        provider_readiness: Vec::new(),
    };
    let items = model_selector_items(&library, Some("fixture-provider"));
    assert_eq!(items.len(), 2_500);

    let list = Arc::new(SelectList::new(items, 12));
    let selector = SearchableSelectList::new(
        Some("Select model"),
        Some("Choose a model from the upstream catalog."),
        Arc::clone(&list),
    );
    selector.set_search_text("fixture-model-2499");

    let selected = list
        .get_selected_item()
        .expect("search result remains selectable");
    assert_eq!(
        selected.value,
        model_row_id("fixture-provider", "fixture-model-2499")
    );
    assert_eq!(selected.label, "Fixture Model 2499");
}
