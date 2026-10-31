//! Checks provider package rows published in the active catalog generation.

use scriberig_app::ProviderExtensionRuntime;

/// Finds the model selected from the current paired catalog generation.
///
/// - `runtime`: Provider/catalog generation source.
///
/// # Returns
/// The fixture model from the active generation.
///
/// # Panics
/// Panics when the active catalog omits the fixture model.
pub(super) fn catalog_model(runtime: &ProviderExtensionRuntime) -> rpi_ai::Model {
    runtime
        .model_catalog()
        .snapshot()
        .models
        .into_iter()
        .find(|model| model.id == "scriberig-fixture-model")
        .expect("resolve fixture model from active catalog")
}

/// Verifies refreshing the fixture package does not duplicate its model row.
///
/// - `runtime`: Shared provider and catalog generation source.
///
/// # Panics
/// Panics when the active catalog has a missing, duplicated, or foreign row.
pub(super) fn assert_single_fixture_catalog_model(runtime: &ProviderExtensionRuntime) {
    let models = runtime.model_catalog().snapshot().models;
    assert_eq!(models.len(), 1, "package refresh duplicated catalog rows");
    assert_eq!(models[0].provider, "scriberig-fixture-provider");
    assert_eq!(models[0].id, "scriberig-fixture-model");
}
