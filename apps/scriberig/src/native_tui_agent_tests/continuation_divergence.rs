//! Covers root continuation reseed for every cached identity component.
//!
//! A cached upstream provider continuation handle stays valid only while the
//! exact provider, model, connection profile, thinking assignment, compiled
//! context, and semantic grants match. Each test changes exactly one of those
//! inputs where the app can vary it independently and requires a fresh provider
//! request.

use std::sync::Arc;

use super::continuation_fixture::{LifecycleFixture, catalog_row, lifecycle_fixture};
use crate::native_tui_agent::continuation_scenario::ContinuationScenarioProvider;

use super::continuation_lifecycle::{
    LATER_MODEL_ID, MODEL_ID, PROVIDER_ID, connection, local_fixture,
};

/// A changed thinking assignment reseeds from canonical context.
#[test]
fn thinking_divergence_reseeds_the_root_continuation() {
    let provider = Arc::new(ContinuationScenarioProvider::new(
        PROVIDER_ID,
        vec![catalog_row(PROVIDER_ID, MODEL_ID)],
    ));
    let selected_connection = connection(PROVIDER_ID, MODEL_ID);
    let fixture = lifecycle_fixture(
        vec![catalog_row(PROVIDER_ID, MODEL_ID)],
        vec![(selected_connection, Arc::clone(&provider))],
        MODEL_ID,
    );
    let context = LifecycleFixture::context(&[]);
    fixture.set_selection(Some(PROVIDER_ID), Some(MODEL_ID), None);

    let first = fixture
        .submit(&context, None, "first instruction")
        .expect("first root turn");
    let root = first.branch.expect("root branch handle");

    fixture.set_selection(Some(PROVIDER_ID), Some(MODEL_ID), Some("high"));
    fixture.reactivate(root);
    fixture
        .submit(&context, Some(root), "second instruction")
        .expect("second root turn after a thinking change");

    let requests = provider.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        !requests[1].0,
        "a changed thinking assignment must not reuse the cached handle"
    );
    fixture.cleanup();
}

/// A changed host connection profile reseeds even for one provider and model.
#[test]
fn connection_profile_divergence_reseeds_the_root_continuation() {
    let selected_connection = connection(PROVIDER_ID, MODEL_ID);
    let local_provider = Arc::new(ContinuationScenarioProvider::new(
        PROVIDER_ID,
        vec![catalog_row(PROVIDER_ID, MODEL_ID)],
    ));
    let selected_provider = Arc::new(ContinuationScenarioProvider::new(
        PROVIDER_ID,
        vec![catalog_row(PROVIDER_ID, MODEL_ID)],
    ));
    let fixture = lifecycle_fixture(
        vec![catalog_row(PROVIDER_ID, MODEL_ID)],
        vec![
            ("local".to_owned(), Arc::clone(&local_provider)),
            (selected_connection, Arc::clone(&selected_provider)),
        ],
        MODEL_ID,
    );
    let context = LifecycleFixture::context(&[]);

    let first = fixture
        .submit(&context, None, "first instruction")
        .expect("first root turn");
    let root = first.branch.expect("root branch handle");
    assert_eq!(fixture.cached_branches(), vec![root]);

    fixture.set_selection(Some(PROVIDER_ID), Some(MODEL_ID), None);
    fixture.reactivate(root);
    fixture
        .submit(&context, Some(root), "second instruction")
        .expect("second root turn after a profile change");

    assert_eq!(local_provider.requests(), vec![(false, None)]);
    assert_eq!(
        selected_provider.requests(),
        vec![(false, None)],
        "a changed connection profile must seed a fresh provider request"
    );
    fixture.cleanup();
}

/// A resolved model change reseeds the root continuation.
///
/// The app resolves a session model change together with its connection
/// identity, so this covers the combined model/profile reseed at the runner.
#[test]
fn model_divergence_reseeds_the_root_continuation() {
    let provider = Arc::new(ContinuationScenarioProvider::new(
        PROVIDER_ID,
        vec![
            catalog_row(PROVIDER_ID, MODEL_ID),
            catalog_row(PROVIDER_ID, LATER_MODEL_ID),
        ],
    ));
    let fixture = lifecycle_fixture(
        vec![
            catalog_row(PROVIDER_ID, MODEL_ID),
            catalog_row(PROVIDER_ID, LATER_MODEL_ID),
        ],
        vec![
            ("local".to_owned(), Arc::clone(&provider)),
            (
                connection(PROVIDER_ID, LATER_MODEL_ID),
                Arc::clone(&provider),
            ),
        ],
        MODEL_ID,
    );
    let context = LifecycleFixture::context(&[]);

    let first = fixture
        .submit(&context, None, "first instruction")
        .expect("first root turn");
    let root = first.branch.expect("root branch handle");
    assert_eq!(fixture.cached_branches(), vec![root]);

    fixture.set_selection(Some(PROVIDER_ID), Some(LATER_MODEL_ID), None);
    fixture.reactivate(root);
    let second = fixture
        .submit(&context, Some(root), "second instruction")
        .expect("second root turn after a model change");
    assert_eq!(second.failure, None);

    assert_eq!(
        provider.requests(),
        vec![(false, None), (false, None)],
        "a changed model assignment must not reuse the cached handle"
    );
    fixture.cleanup();
}

/// A different provider for the same model id reseeds the root continuation.
#[test]
fn provider_divergence_reseeds_the_root_continuation() {
    let first_provider = Arc::new(ContinuationScenarioProvider::new(
        "continuation-a",
        vec![catalog_row("continuation-a", MODEL_ID)],
    ));
    let second_provider = Arc::new(ContinuationScenarioProvider::new(
        "continuation-b",
        vec![catalog_row("continuation-b", MODEL_ID)],
    ));
    let catalog = vec![
        catalog_row("continuation-a", MODEL_ID),
        catalog_row("continuation-b", MODEL_ID),
    ];
    let fixture = lifecycle_fixture(
        catalog,
        vec![
            (
                connection("continuation-a", MODEL_ID),
                Arc::clone(&first_provider),
            ),
            (
                connection("continuation-b", MODEL_ID),
                Arc::clone(&second_provider),
            ),
        ],
        MODEL_ID,
    );
    let context = LifecycleFixture::context(&[]);
    fixture.set_selection(Some("continuation-a"), Some(MODEL_ID), None);

    let first = fixture
        .submit(&context, None, "first instruction")
        .expect("first root turn with provider a");
    let root = first.branch.expect("root branch handle");
    assert_eq!(fixture.cached_branches(), vec![root]);

    fixture.set_selection(Some("continuation-b"), Some(MODEL_ID), None);
    fixture.reactivate(root);
    fixture
        .submit(&context, Some(root), "second instruction")
        .expect("second root turn with provider b");

    let a_requests = first_provider.requests();
    let b_requests = second_provider.requests();
    assert_eq!(a_requests.len(), 1);
    assert_eq!(b_requests.len(), 1);
    assert!(
        !b_requests[0].0,
        "a different provider must not receive the previous provider's handle"
    );
    fixture.cleanup();
}

/// A durable context change reseeds the root continuation.
#[test]
fn context_divergence_reseeds_the_root_continuation() {
    let fixture = local_fixture();
    let context = LifecycleFixture::context(&[]);

    let first = fixture
        .submit(&context, None, "first instruction")
        .expect("first root turn");
    let root = first.branch.expect("root branch handle");

    fixture.append_turn(root, "externally added turn", "external answer");
    fixture.reactivate(root);
    fixture
        .submit(&context, Some(root), "second instruction")
        .expect("second root turn after a context change");

    let requests = fixture.provider.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        !requests[1].0,
        "a changed compiled context must not reuse the cached handle"
    );
    fixture.cleanup();
}

/// A changed semantic grant set reseeds the root continuation.
#[test]
fn capability_divergence_reseeds_the_root_continuation() {
    let fixture = local_fixture();

    let first = fixture
        .submit(&LifecycleFixture::context(&[]), None, "first instruction")
        .expect("first root turn");
    let root = first.branch.expect("root branch handle");

    fixture.reactivate(root);
    fixture
        .submit(
            &LifecycleFixture::context(&["knowledge.write"]),
            Some(root),
            "second instruction",
        )
        .expect("second root turn with an extra grant");

    let requests = fixture.provider.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        !requests[1].0,
        "a changed capability set must not reuse the cached handle"
    );
    fixture.cleanup();
}
