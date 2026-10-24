//! Covers the `ScribeRig`-owned ephemeral continuation lifecycle decisions.
//!
//! The owner decides whether an opaque upstream handle may be reused. These
//! tests pin that decision at the layer that owns it: a fresh execution seeds
//! from canonical compiled context, a same-lineage execution reuses the retained
//! handle through the real provider request, divergence in any binding component
//! reseeds, and a failed, cancelled, or handle-less completion clears it.
//! Sibling isolation, scope rejection, restart seeding, and non-persistence are
//! in `owner_scope`.

use std::sync::Arc;

use rpi_ai::{Model, Provider, ProviderContinuation, ThinkingLevel};
use scriberig_agent_runtime::continuation::{
    CompletedContinuation, ContinuationBinding, ContinuationOutcome, EphemeralContinuationOwner,
};
use scriberig_core::EntityId;

use super::fixture::{
    ContinuationProvider, FixtureContinuationState, fixture_model, invocation_branch, run,
};

pub(super) const PROVIDER_ID: &str = "continuation-fixture";
pub(super) const MODEL_ID: &str = "continuation-model";
const PROFILE: &str = "host-profile";
const FINGERPRINT: &str = "context-fingerprint-v1";

/// Returns the canonical execution identity the tests bind handles to.
///
/// # Returns
/// The branch identity the fixture invocation runs as.
pub(super) fn branch() -> EntityId {
    invocation_branch()
}

/// Returns the exact execution identity a handle is bound to.
///
/// # Returns
/// A binding for the fixture provider, model, profile, and context fingerprint.
pub(super) fn binding() -> ContinuationBinding {
    ContinuationBinding::new(
        PROVIDER_ID,
        fixture_model(PROVIDER_ID, MODEL_ID),
        PROFILE,
        ThinkingLevel::Off,
        FINGERPRINT,
    )
}

/// Returns a copy of the canonical binding changed in exactly one component.
///
/// # Parameters
/// - `adjust`: Mutation that makes the binding diverge from the canonical one.
///
/// # Returns
/// The divergent binding to revalidate a retained handle against.
pub(super) fn divergent_binding(
    adjust: impl FnOnce(&mut ContinuationBinding),
) -> ContinuationBinding {
    let mut candidate = binding();
    adjust(&mut candidate);
    candidate
}

/// Builds one opaque handle carrying private provider state named by `marker`.
///
/// # Parameters
/// - `marker`: Private provider state value the fixture reads back on use.
/// - `model_id`: Model identity the handle is scoped to.
///
/// # Returns
/// A provider/model-scoped handle with opaque state.
pub(super) fn handle_for(marker: &str, model_id: &str) -> ProviderContinuation {
    ProviderContinuation::new(
        PROVIDER_ID,
        model_id,
        FixtureContinuationState {
            response_id: marker.to_owned(),
        },
    )
}

/// Retains one completed handle for the canonical execution identity.
///
/// # Parameters
/// - `owner`: Owner under test.
/// - `provider`: Provider that completed the attempt.
/// - `handle`: Handle extracted from the completed response.
///
/// # Returns
/// Whether the owner retained the handle.
pub(super) fn retain(
    owner: &EphemeralContinuationOwner,
    provider: &dyn Provider,
    handle: ProviderContinuation,
) -> bool {
    owner.settle(
        provider,
        branch(),
        ContinuationOutcome::Completed(Box::new(CompletedContinuation {
            binding: binding(),
            handle,
        })),
    )
}

/// Runs one cold invocation and returns its extracted handle.
///
/// # Parameters
/// - `provider`: Fixture provider serving the invocation.
/// - `model`: Exact model bound to the invocation.
///
/// # Returns
/// The opaque handle the upstream loop extracted.
pub(super) fn cold_handle(provider: Arc<dyn Provider>, model: Model) -> ProviderContinuation {
    let (_, extracted) = run(provider, model, None).expect("cold invocation");
    extracted.expect("extracted continuation")
}

/// Builds a fixture provider that advertises continuation.
///
/// # Parameters
/// - `served`: Model rows the fixture serves.
///
/// # Returns
/// A continuation-capable fixture provider with an empty request log.
pub(super) fn continuation_provider(served: Vec<Model>) -> Arc<ContinuationProvider> {
    Arc::new(ContinuationProvider::new(PROVIDER_ID, served, true))
}

/// A fresh execution has no handle, seeds from canonical context, and retains
/// exactly one handle after its completed turn.
#[test]
fn a_fresh_execution_seeds_from_canonical_context() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![model.clone()]);
    let owner = EphemeralContinuationOwner::new();
    assert!(owner.is_empty(), "a fresh owner holds no handle");
    assert!(
        owner
            .take_if_current(provider.as_ref(), branch(), &binding())
            .is_none(),
        "the first attempt must seed from canonical compiled context"
    );

    let extracted = cold_handle(Arc::clone(&provider) as Arc<dyn Provider>, model.clone());
    assert!(
        retain(&owner, provider.as_ref(), extracted),
        "a completed turn retains its handle"
    );
    assert_eq!(owner.branches(), vec![branch()]);
    let observed = provider.observed.lock().expect("request log");
    assert_eq!(
        observed.len(),
        1,
        "the fresh execution dispatched exactly one provider request"
    );
    assert!(
        !observed[0].continued,
        "a fresh execution must send no handle"
    );
}

/// A same-lineage execution reuses the retained handle and continues the exact
/// prior provider response.
#[test]
fn a_same_lineage_execution_reuses_the_retained_handle() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![model.clone()]);
    let provider_dyn = Arc::clone(&provider) as Arc<dyn Provider>;
    let owner = EphemeralContinuationOwner::new();

    let first = cold_handle(Arc::clone(&provider_dyn), model.clone());
    assert!(
        retain(&owner, provider.as_ref(), first.clone()),
        "the completed turn retains its handle"
    );

    let reused = owner
        .take_if_current(provider.as_ref(), branch(), &binding())
        .expect("same-lineage handle");
    let (_, second) = run(provider_dyn, model.clone(), Some(reused)).expect("continued invocation");

    let observed = provider.observed.lock().expect("request log");
    assert_eq!(observed.len(), 2);
    assert!(
        observed[1].continued,
        "the reused handle must reach the provider continuation request"
    );
    assert_eq!(
        observed[1].continued_response_id.as_deref(),
        Some(observed[0].response_id.as_str()),
        "the reused handle must continue the exact prior response"
    );
    assert!(
        second.expect("continued response handle") != first,
        "a continued response yields its own handle"
    );
    assert!(
        owner.is_empty(),
        "a consumed handle is not reusable until the attempt settles"
    );
}

/// Divergence in any single binding component reseeds and drops the stale handle.
#[test]
fn binding_divergence_reseeds_and_drops_the_stale_handle() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![
        model.clone(),
        fixture_model(PROVIDER_ID, "continuation-model-later"),
    ]);
    let owner = EphemeralContinuationOwner::new();

    let divergent = [
        (
            "provider",
            divergent_binding(|candidate| {
                candidate.provider_id = "other-continuation-fixture".to_owned();
            }),
        ),
        (
            "model",
            divergent_binding(|candidate| {
                candidate.model = fixture_model(PROVIDER_ID, "continuation-model-later");
            }),
        ),
        (
            "profile",
            divergent_binding(|candidate| {
                candidate.profile = "other-host-profile".to_owned();
            }),
        ),
        (
            "thinking level",
            divergent_binding(|candidate| {
                candidate.thinking_level = ThinkingLevel::Medium;
            }),
        ),
        (
            "context fingerprint",
            divergent_binding(|candidate| {
                candidate.context_fingerprint = "other-context-fingerprint".to_owned();
            }),
        ),
    ];
    for (component, candidate) in divergent {
        assert!(
            retain(
                &owner,
                provider.as_ref(),
                handle_for("response-divergence", MODEL_ID)
            ),
            "{component} case must start with a retained handle"
        );
        assert!(
            owner
                .take_if_current(provider.as_ref(), branch(), &candidate)
                .is_none(),
            "a {component} change must reseed from canonical context"
        );
        assert!(
            owner.is_empty(),
            "a {component} change must drop the stale handle"
        );
    }
}

/// Failed, cancelled, and handle-less completions leave nothing retained.
#[test]
fn failure_cancellation_and_missing_handles_clear_the_owner() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![model.clone()]);
    let owner = EphemeralContinuationOwner::new();

    for (label, outcome) in [
        ("failure", ContinuationOutcome::Failed),
        ("cancellation", ContinuationOutcome::Cancelled),
        ("missing handle", ContinuationOutcome::NoHandle),
    ] {
        assert!(
            retain(
                &owner,
                provider.as_ref(),
                handle_for("response-outcome", MODEL_ID)
            ),
            "{label} case must start with a retained handle"
        );
        assert!(
            !owner.settle(provider.as_ref(), branch(), outcome),
            "a {label} attempt must retain nothing"
        );
        assert!(owner.is_empty(), "a {label} attempt must clear the owner");
    }

    assert!(
        !owner.settle(provider.as_ref(), branch(), ContinuationOutcome::Failed),
        "an attempt with no retained handle still retains nothing"
    );
    assert!(
        retain(
            &owner,
            provider.as_ref(),
            handle_for("response-outcome", MODEL_ID)
        ),
        "the discard case must start with a retained handle"
    );
    assert!(
        owner.discard(branch()),
        "an explicit discard drops the retained handle"
    );
    assert!(owner.is_empty(), "an explicit discard clears the owner");
    assert!(
        !owner.discard(branch()),
        "discarding an already clear identity changes nothing"
    );
}
