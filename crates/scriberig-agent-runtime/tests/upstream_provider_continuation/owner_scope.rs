//! Covers owner isolation, scope rejection, restart seeding, and persistence.
//!
//! A retained handle belongs to exactly one `ScribeRig` execution identity: it is
//! never handed to a sibling identity, never retained for a provider that does
//! not advertise continuation or for a model outside its binding, never survives
//! a process restart, and never reaches a durable request projection or a
//! diagnostic rendering. Lifecycle seeding, reuse, divergence, and clearing are
//! in `owner`.

use std::sync::Arc;

use rpi_ai::Provider;
use scriberig_agent_runtime::continuation::{
    CompletedContinuation, ContinuationOutcome, EphemeralContinuationOwner,
};
use scriberig_core::EntityId;

use super::fixture::{fixture_model, run};
use super::owner::{
    MODEL_ID, PROVIDER_ID, binding, branch, cold_handle, continuation_provider, handle_for, retain,
};

/// A sibling execution never receives another execution's handle.
#[test]
fn sibling_executions_never_share_a_retained_handle() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![model.clone()]);
    let owner = EphemeralContinuationOwner::new();
    let sibling = EntityId::from_bytes([92; 16]);
    assert!(
        retain(
            &owner,
            provider.as_ref(),
            handle_for("response-sibling", MODEL_ID)
        ),
        "the root execution retains its handle"
    );

    assert!(
        owner
            .take_if_current(provider.as_ref(), sibling, &binding())
            .is_none(),
        "a sibling identity must not receive the root handle"
    );
    assert_eq!(
        owner.branches(),
        vec![branch()],
        "a sibling lookup must not move or clone the retained handle"
    );
    assert!(
        owner
            .take_if_current(provider.as_ref(), branch(), &binding())
            .is_some(),
        "the owning identity still receives its own handle"
    );
}

/// An unsupported provider or a handle outside its binding is never retained.
#[test]
fn an_unsupported_provider_or_out_of_scope_handle_is_not_retained() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let later = fixture_model(PROVIDER_ID, "continuation-model-later");
    let supported = continuation_provider(vec![model.clone(), later.clone()]);
    let unsupported = Arc::new(super::fixture::ContinuationProvider::new(
        "unsupported-continuation-fixture",
        vec![model.clone()],
        false,
    ));
    let owner = EphemeralContinuationOwner::new();

    assert!(
        !owner.settle(
            unsupported.as_ref(),
            branch(),
            ContinuationOutcome::Completed(Box::new(CompletedContinuation {
                binding: binding(),
                handle: handle_for("response-unsupported", MODEL_ID),
            })),
        ),
        "a provider without continuation support retains nothing"
    );

    assert!(
        !owner.settle(
            supported.as_ref(),
            branch(),
            ContinuationOutcome::Completed(Box::new(CompletedContinuation {
                binding: binding(),
                handle: handle_for("response-mismatch", "continuation-model-later"),
            })),
        ),
        "a handle outside its binding's provider/model scope retains nothing"
    );
    assert!(owner.is_empty(), "neither rejected handle may be reused");
}

/// A restarted owner holds nothing, so the next execution seeds from canonical
/// context instead of any previous handle.
#[test]
fn a_restarted_owner_seeds_from_canonical_context() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![model.clone()]);
    let provider_dyn = Arc::clone(&provider) as Arc<dyn Provider>;
    let previous = EphemeralContinuationOwner::new();
    let first = cold_handle(Arc::clone(&provider_dyn), model.clone());
    assert!(
        retain(&previous, provider.as_ref(), first),
        "the pre-restart execution retains its handle"
    );

    let restarted = EphemeralContinuationOwner::new();
    let candidate = restarted.take_if_current(provider.as_ref(), branch(), &binding());
    assert!(
        candidate.is_none(),
        "a restarted owner must hold no provider handle"
    );
    run(provider_dyn, model.clone(), candidate).expect("restarted invocation");
    let observed = provider.observed.lock().expect("request log");
    assert_eq!(observed.len(), 2);
    assert!(
        !observed[1].continued,
        "the restarted execution must seed from canonical compiled context"
    );
}

/// Handle material reaches the provider request but never a durable projection
/// or a diagnostic rendering.
#[test]
fn handle_material_stays_out_of_durable_projections_and_diagnostics() {
    const MARKER: &str = "opaque-handle-material";
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![model.clone()]);
    let owner = EphemeralContinuationOwner::new();
    let handle = handle_for(MARKER, MODEL_ID);
    assert!(
        retain(&owner, provider.as_ref(), handle.clone()),
        "the completed turn retains its handle"
    );
    assert!(
        !format!("{owner:?}").contains(MARKER)
            && !format!("{:?}", binding()).contains(MARKER)
            && !format!("{handle:?}").contains(MARKER),
        "diagnostics must keep provider handle state opaque"
    );

    let reused = owner
        .take_if_current(provider.as_ref(), branch(), &binding())
        .expect("retained handle");
    let (result, _) = run(
        Arc::clone(&provider) as Arc<dyn Provider>,
        model.clone(),
        Some(reused),
    )
    .expect("continued invocation");

    let observed = provider.observed.lock().expect("request log");
    assert_eq!(
        observed.len(),
        1,
        "the retained handle drives exactly one continued provider request"
    );
    assert!(
        observed[0].continued,
        "the retained handle must reach the provider continuation request"
    );
    assert_eq!(
        observed[0].continued_response_id.as_deref(),
        Some(MARKER),
        "the provider must have consumed the handle material"
    );
    let durable = serde_json::to_string(&result.model_requests).expect("serialize projections");
    assert!(
        !durable.contains(MARKER),
        "provider handle material leaked into a durable request projection"
    );
}
