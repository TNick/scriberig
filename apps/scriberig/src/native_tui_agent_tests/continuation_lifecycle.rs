//! Covers the native root runner's ephemeral continuation handle lifecycle.
//!
//! `ScribeRig` owns the branch/context validity decision for upstream provider
//! continuation handles. These tests pin that ownership at the app boundary that
//! holds the cache: same-lineage root reuse, failed and cancelled
//! non-reinsertion, and restart statelessness. Per-component divergence reseed
//! lives in `continuation_divergence`. No provider handle material may reach
//! durable session state.

use std::sync::Arc;

use scriberig_tui::AgentCommandRunner;

use super::continuation_fixture::{LifecycleFixture, catalog_row, lifecycle_fixture};
use crate::native_tui_agent::continuation_scenario::{
    ContinuationScenarioProvider, ScenarioOutcome,
};

pub(super) const PROVIDER_ID: &str = "continuation-scenario";
pub(super) const MODEL_ID: &str = "fake-model";
pub(super) const LATER_MODEL_ID: &str = "fake-model-later";

/// Returns the host connection identity for one selected provider model.
///
/// # Parameters
/// - `provider_id`: Exact upstream provider identity.
/// - `model_id`: Provider-native model identity.
///
/// # Returns
/// The connection key the native runner resolves for that selection.
pub(super) fn connection(provider_id: &str, model_id: &str) -> String {
    crate::native_tui_agent::provider_config::catalog::connection_identity(provider_id, model_id)
}

/// Builds one single-provider fixture with the provider on the local connection.
///
/// # Returns
/// A fixture whose catalog and connection bindings use `PROVIDER_ID`.
pub(super) fn local_fixture() -> LifecycleFixture {
    let provider = Arc::new(ContinuationScenarioProvider::new(
        PROVIDER_ID,
        vec![
            catalog_row(PROVIDER_ID, MODEL_ID),
            catalog_row(PROVIDER_ID, LATER_MODEL_ID),
        ],
    ));
    lifecycle_fixture(
        vec![
            catalog_row(PROVIDER_ID, MODEL_ID),
            catalog_row(PROVIDER_ID, LATER_MODEL_ID),
        ],
        vec![("local".to_owned(), provider)],
        MODEL_ID,
    )
}

/// Reuses one cached handle for the same root lineage and reseeds on divergence.
#[test]
fn same_lineage_root_turn_reuses_the_cached_continuation_handle() {
    let fixture = local_fixture();
    let context = LifecycleFixture::context(&[]);

    let first = fixture
        .submit(&context, None, "first instruction")
        .expect("first root turn");
    let root = first.branch.expect("root branch handle");
    assert_eq!(fixture.cached_branches(), vec![root]);

    fixture.reactivate(root);
    let second = fixture
        .submit(&context, Some(root), "second instruction")
        .expect("second root turn on the same lineage");
    assert_eq!(
        second.branch,
        Some(root),
        "an active root branch keeps its canonical identity"
    );

    let requests = fixture.provider.requests();
    assert_eq!(requests.len(), 2);
    assert!(!requests[0].0, "the cold turn must not send a handle");
    assert!(
        requests[1].0,
        "the same lineage must reuse the cached provider continuation"
    );
    assert_eq!(
        requests[1].1.as_deref(),
        fixture.provider.response_marker(0).as_deref(),
        "the reused handle must continue the exact prior response"
    );
    assert_eq!(fixture.cached_branches(), vec![root]);
    fixture.cleanup();
}

/// Never persists or projects provider continuation handle material.
#[test]
fn cached_continuation_handles_stay_out_of_durable_session_state() {
    let fixture = local_fixture();
    let context = LifecycleFixture::context(&[]);

    let first = fixture
        .submit(&context, None, "first instruction")
        .expect("first root turn");
    let root = first.branch.expect("root branch handle");
    fixture.reactivate(root);
    fixture
        .submit(&context, Some(root), "second instruction")
        .expect("continued root turn");

    let durable = fixture.durable();
    let serialized = serde_json::to_string(&durable).expect("serialize session");
    assert!(
        !serialized.contains("opaque-continuation"),
        "provider continuation marker leaked into durable session state"
    );
    assert!(
        durable.branches.iter().any(|branch| {
            branch.turns.iter().any(|turn| {
                turn.agent
                    .as_ref()
                    .is_some_and(|answer| answer.text == "fake-model answer")
            })
        }),
        "the continued turn must still settle its ScribeRig result"
    );
    fixture.cleanup();
}

/// A failed root turn leaves no cached handle for the next turn.
#[test]
fn failed_root_turn_does_not_reinsert_a_continuation_handle() {
    let fixture = local_fixture();
    let context = LifecycleFixture::context(&[]);
    fixture
        .provider
        .queue_outcomes(vec![ScenarioOutcome::Failed]);

    let failure = fixture
        .submit(&context, None, "first instruction")
        .expect_err("failed provider turn");
    assert!(
        matches!(
            failure,
            scriberig_app::ApplicationError::OperationFailed { .. }
        ),
        "failed root turn: {failure:?}"
    );
    assert!(
        fixture.cached_branches().is_empty(),
        "a failed root turn must not cache a continuation handle"
    );
    assert_eq!(fixture.provider.requests().len(), 1);
    fixture.cleanup();
}

/// A provider-aborted root turn leaves no cached handle for the next turn.
#[test]
fn cancelled_root_turn_does_not_reinsert_a_continuation_handle() {
    let fixture = local_fixture();
    let context = LifecycleFixture::context(&[]);
    fixture
        .provider
        .queue_outcomes(vec![ScenarioOutcome::Aborted]);

    let failure = fixture
        .submit(&context, None, "first instruction")
        .expect_err("aborted provider turn");
    let message = failure.to_string();
    assert!(
        message.contains("cancel") || message.contains("abort"),
        "aborted root turn: {failure:?}"
    );
    assert!(
        fixture.cached_branches().is_empty(),
        "a cancelled root turn must not cache a continuation handle"
    );
    fixture.cleanup();
}

/// A cancelled operation keeps no handle even when its provider turn ran.
///
/// The provider response normally settles before the runner's 25 ms
/// cancellation poll, so the post-completion cancellation guard is what rejects
/// the handle; whichever path settles first, no entry may survive.
#[test]
fn cancelled_operation_does_not_cache_a_handle_after_a_provider_turn() {
    let fixture = local_fixture();
    let context = LifecycleFixture::context(&[]);
    let accepted = fixture
        .service
        .begin_agent_execution(
            &context,
            &scriberig_app::AgentCommandRequest {
                session: fixture.session,
                branch: None,
                anchor: None,
                prompt: "first instruction".to_owned(),
                selection: None,
            },
        )
        .expect("accepted root turn");
    fixture
        .service
        .operations()
        .cancellation_flag(&accepted.operation)
        .expect("operation cancellation flag")
        .store(true, std::sync::atomic::Ordering::Release);

    let _ = fixture
        .runner
        .execute_accepted(&fixture.service, &context, &accepted);
    assert_eq!(
        fixture.provider.requests().len(),
        1,
        "the cancelled operation still attempted one provider request"
    );
    assert!(
        fixture.cached_branches().is_empty(),
        "a cancelled operation must not cache a continuation handle"
    );
    fixture.cleanup();
}

/// A restarted runner seeds from canonical context instead of any handle.
#[test]
fn restart_reseeds_the_root_continuation_from_canonical_context() {
    let fixture = local_fixture();
    let context = LifecycleFixture::context(&[]);

    let first = fixture
        .submit(&context, None, "first instruction")
        .expect("first root turn");
    let root = first.branch.expect("root branch handle");
    assert_eq!(fixture.cached_branches(), vec![root]);

    fixture.reactivate(root);
    let restarted = fixture.fresh_runner();
    restarted
        .submit(
            &fixture.service,
            &context,
            fixture.session,
            Some(root),
            "second instruction",
        )
        .expect("restarted root turn");

    let requests = fixture.provider.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        !requests[1].0,
        "a restarted runner must seed from canonical ScribeRig context"
    );
    assert!(
        fixture.cached_branches() == vec![root] || fixture.cached_branches().is_empty(),
        "the restarted runner owns its own ephemeral cache"
    );
    fixture.cleanup();
}
