//! Pins the provider-identity half of the host reseed decision.
//!
//! A handle belongs to the provider that produced it and to a provider that
//! advertises audited continuation. These cases pin both provider-side refusals
//! at the invocation seam, and then pin that the refused attempt still runs the
//! branch from canonical compiled `ScribeRig` context instead of failing. The
//! attempt helpers are shared from `reseed_lineage`.

use std::sync::Arc;

use rpi_ai::Provider;
use scriberig_agent_runtime::continuation::{
    CompletedContinuation, ContinuationOutcome, EphemeralContinuationOwner,
};
use scriberig_agent_runtime::upstream_invocation::UpstreamBranchError;

use super::fixture::{ContinuationProvider, answer, fixture_model};
use super::owner::{MODEL_ID, PROVIDER_ID, branch, continuation_provider, handle_for};
use super::reseed_lineage::{
    COMPLETED_LINEAGE, HANDLE_MARKER, binding_for, compiled_context, dispatched_fingerprint,
    retain_root_handle, run_on,
};

/// Provider identity of the second fixture provider.
const SECONDARY_PROVIDER: &str = "continuation-fixture-secondary";
/// Upstream rejection the invocation seam reports for an out-of-scope handle.
const SCOPE_REJECTION: &str = "provider continuation is outside the selected provider/model scope";

/// A handle one provider produced is refused by another provider for the exact
/// same model row, before any provider request.
#[test]
fn a_handle_from_another_provider_is_refused_before_dispatch() {
    let model_a = fixture_model(PROVIDER_ID, MODEL_ID);
    let model_b = fixture_model(SECONDARY_PROVIDER, MODEL_ID);
    let provider_a = continuation_provider(vec![model_a.clone()]);
    let provider_b = Arc::new(ContinuationProvider::new(
        SECONDARY_PROVIDER,
        vec![model_b.clone()],
        true,
    ));

    let failure = run_on(
        Arc::clone(&provider_b) as Arc<dyn Provider>,
        &model_b,
        branch(),
        compiled_context(branch(), COMPLETED_LINEAGE),
        Some(handle_for(HANDLE_MARKER, MODEL_ID)),
    )
    .expect_err("handle scoped to another provider");
    assert_eq!(
        failure.error,
        UpstreamBranchError::Agent(SCOPE_REJECTION.to_owned())
    );
    assert!(
        failure.model_requests.is_empty(),
        "the refused attempt captures no durable request"
    );
    assert!(
        provider_b.observed.lock().expect("request log").is_empty(),
        "the provider that did not produce the handle must not be contacted"
    );

    let owner = EphemeralContinuationOwner::new();
    assert!(
        retain_root_handle(&owner, provider_a.as_ref(), &model_a, COMPLETED_LINEAGE),
        "the producing provider's completed turn retains its handle"
    );
    assert!(
        owner
            .take_if_current(
                provider_b.as_ref(),
                branch(),
                &binding_for(SECONDARY_PROVIDER, &model_b, COMPLETED_LINEAGE),
            )
            .is_none(),
        "a provider switch must never be offered the previous provider's handle"
    );
    assert!(
        owner.is_empty(),
        "the refused provider switch drops the retained handle so the next attempt seeds from canonical context"
    );
}

/// A provider that stops advertising continuation still completes the branch
/// from canonical context, and its completion retains nothing for a later attempt.
///
/// The second provider reports the same provider id and serves the same model row
/// as the first, so the advertised capability is the only thing that can reject
/// the retained handle; provider/model scope alone would accept it.
#[test]
fn an_unsupported_provider_completes_the_branch_from_canonical_context() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let supported = continuation_provider(vec![model.clone()]);
    let unsupported = Arc::new(ContinuationProvider::new(
        PROVIDER_ID,
        vec![model.clone()],
        false,
    ));
    let owner = EphemeralContinuationOwner::new();

    let cold = run_on(
        Arc::clone(&supported) as Arc<dyn Provider>,
        &model,
        branch(),
        compiled_context(branch(), COMPLETED_LINEAGE),
        None,
    )
    .expect("cold attempt on the supported provider");
    let lineage = dispatched_fingerprint(&cold.0);
    let retained = cold.1.expect("extracted handle");
    assert!(
        retained.is_scoped_to(PROVIDER_ID, &model.id),
        "the retained handle is inside the unsupported selection's provider/model scope, so only the advertised capability can reject it"
    );
    assert!(
        owner.settle(
            supported.as_ref(),
            branch(),
            ContinuationOutcome::Completed(Box::new(CompletedContinuation {
                binding: binding_for(PROVIDER_ID, &model, &lineage),
                handle: retained,
            })),
        ),
        "the supported provider's completed turn retains its handle"
    );

    // The host resolves the same branch lineage, provider id, and model row to a
    // provider that advertises no continuation; that selection is a reseed, never
    // an error.
    let candidate = owner.take_if_current(
        unsupported.as_ref(),
        branch(),
        &binding_for(PROVIDER_ID, &model, &lineage),
    );
    assert!(
        candidate.is_none(),
        "a provider without continuation support is offered no handle"
    );
    assert!(
        owner.is_empty(),
        "the unsupported selection drops the handle the previous provider produced"
    );

    let replayed = run_on(
        Arc::clone(&unsupported) as Arc<dyn Provider>,
        &model,
        branch(),
        compiled_context(branch(), COMPLETED_LINEAGE),
        candidate,
    )
    .expect("unsupported provider attempt");
    assert_eq!(
        replayed.0.output,
        answer(&model.id),
        "the branch settles without upstream continuation"
    );
    assert!(
        replayed.1.is_none(),
        "an unsupported provider extracts no handle to retain"
    );
    assert_eq!(
        dispatched_fingerprint(&replayed.0),
        lineage,
        "the replayed attempt dispatches the same canonical compiled context"
    );
    assert!(
        !owner.settle(
            unsupported.as_ref(),
            branch(),
            ContinuationOutcome::NoHandle
        ),
        "the unsupported completion retains nothing"
    );
    let observed = unsupported.observed.lock().expect("request log");
    assert_eq!(
        observed.len(),
        1,
        "the replay dispatched one provider request"
    );
    assert!(
        !observed[0].continued,
        "the replay sends canonical context, not a handle"
    );
}
