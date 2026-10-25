//! Pins that a retained handle belongs to exactly one branch scope.
//!
//! A delegated child branch and a durable plan worker are separate `ScribeRig`
//! execution identities, and neither may read or replace the handle the root
//! lineage retained. These cases pin both halves of that rule: the owner never
//! offers the root handle to a child or plan-worker identity, and the stateless
//! child and plan-worker dispatch seam sends no handle and leaves nothing
//! retained under any identity, so the root's own next attempt still continues
//! its exact prior provider response. The scoped contexts deliberately compile
//! the root's lineage text, so these cases also pin that the compiled-context
//! fingerprint is not the scope boundary: only the owner's execution identity is.

use std::sync::Arc;

use rpi_ai::{Model, Provider};
use scriberig_agent_runtime::continuation::{
    CompletedContinuation, ContinuationOutcome, EphemeralContinuationOwner,
};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchFailure, UpstreamBranchResult, run_upstream_branch_invocation,
};
use scriberig_core::EntityId;
use tokio_util::sync::CancellationToken;

use super::fixture::{answer, fixture_model};
use super::owner::{MODEL_ID, PROVIDER_ID, branch, continuation_provider, handle_for};
use super::reseed_lineage::{
    COMPLETED_LINEAGE, HANDLE_MARKER, binding_for, compiled_context, dispatched_fingerprint,
    invocation_on, retain_root_handle, run_on,
};

/// Returns the delegated child branch identity.
///
/// # Returns
/// A branch identity distinct from the root branch and from any plan worker.
fn child_branch() -> EntityId {
    EntityId::from_bytes([93; 16])
}

/// Returns the durable plan-worker branch identity.
///
/// # Returns
/// A branch identity distinct from the root branch and from any delegated child.
fn plan_worker_branch() -> EntityId {
    EntityId::from_bytes([94; 16])
}

/// Runs one stateless child or plan-worker dispatch.
///
/// The host's delegated-child and precompiled plan-worker paths pass no handle to
/// the invocation seam and receive none back; `run_upstream_branch_invocation` is
/// that handle-free entry point, so a scoped attempt cannot carry or return one.
///
/// # Parameters
/// - `provider`: Fixture provider the scoped execution runs on.
/// - `model`: Exact model bound to the scoped execution.
/// - `branch`: Child or plan-worker branch identity.
///
/// # Returns
/// The durable scoped execution result, or the invocation failure.
///
/// # Errors
/// Returns the branch invocation failure when the scoped dispatch fails.
fn dispatch_scoped(
    provider: Arc<dyn Provider>,
    model: &Model,
    branch: EntityId,
) -> Result<UpstreamBranchResult, UpstreamBranchFailure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("scoped dispatch runtime");
    runtime.block_on(run_upstream_branch_invocation(
        invocation_on(
            provider,
            model,
            branch,
            compiled_context(branch, COMPLETED_LINEAGE),
        ),
        "continue the same branch",
        CancellationToken::new(),
        None,
    ))
}

/// A child or plan-worker attempt is never offered the root lineage's handle, and
/// its own dispatch keeps handle material out of its durable provenance.
#[test]
fn a_child_or_plan_worker_attempt_is_never_offered_the_root_handle() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![model.clone()]);
    let provider_dyn = Arc::clone(&provider) as Arc<dyn Provider>;
    let owner = EphemeralContinuationOwner::new();
    let cold = run_on(
        Arc::clone(&provider_dyn),
        &model,
        branch(),
        compiled_context(branch(), COMPLETED_LINEAGE),
        None,
    )
    .expect("cold root attempt");
    let binding = binding_for(PROVIDER_ID, &model, &dispatched_fingerprint(&cold.0));
    assert!(
        owner.settle(
            provider.as_ref(),
            branch(),
            ContinuationOutcome::Completed(Box::new(CompletedContinuation {
                binding: binding.clone(),
                handle: handle_for(HANDLE_MARKER, MODEL_ID),
            })),
        ),
        "the completed root turn retains its handle"
    );

    for (label, scoped) in [
        ("delegated child", child_branch()),
        ("durable plan worker", plan_worker_branch()),
    ] {
        assert!(
            owner
                .take_if_current(provider.as_ref(), scoped, &binding)
                .is_none(),
            "the {label} identity must never be offered the root lineage's handle"
        );
        assert_eq!(
            owner.branches(),
            vec![branch()],
            "a {label} lookup must not move or copy the root handle"
        );
    }

    let mut scoped_projections = Vec::new();
    for scoped in [child_branch(), plan_worker_branch()] {
        let dispatch = dispatch_scoped(Arc::clone(&provider_dyn), &model, scoped)
            .expect("stateless scoped dispatch");
        scoped_projections.extend(dispatch.model_requests);
    }
    let observed = provider.observed.lock().expect("request log");
    assert_eq!(
        observed.len(),
        3,
        "the root cold turn plus the child and plan-worker dispatches are the only requests"
    );
    assert!(
        observed[1..].iter().all(|request| !request.continued),
        "child and plan-worker dispatch must seed from canonical ScribeRig context"
    );
    drop(observed);

    let durable = serde_json::to_string(&scoped_projections).expect("serialize scoped projections");
    assert!(
        !durable.contains(HANDLE_MARKER),
        "root handle material reached a scoped execution's durable provenance"
    );
    assert!(
        !format!("{owner:?}").contains(HANDLE_MARKER),
        "diagnostics must keep the retained handle opaque"
    );

    let reused = owner
        .take_if_current(provider.as_ref(), branch(), &binding)
        .expect("the root identity still owns its handle");
    let continued = run_on(
        provider_dyn,
        &model,
        branch(),
        compiled_context(branch(), COMPLETED_LINEAGE),
        Some(reused),
    )
    .expect("continued root attempt");
    assert_eq!(continued.0.output, answer(&model.id));
    let observed = provider.observed.lock().expect("request log");
    let last = observed.last().expect("continued root request");
    assert!(
        last.continued,
        "the scoped runs must not consume the root lineage's handle"
    );
    assert_eq!(
        last.continued_response_id.as_deref(),
        Some(HANDLE_MARKER),
        "the root attempt continues its own exact prior response"
    );
}

/// A scoped execution never inherits a handle, even when it compiles the exact
/// same canonical lineage as the retained one.
///
/// The scoped contexts deliberately carry the root's lineage text, so the
/// compiled-context fingerprint the scoped dispatches produce is identical to the
/// one the root handle is bound to. Isolation therefore cannot rest on context
/// identity or provider/model scope; only the `ScribeRig` execution identity the
/// owner keys on separates them, and a scoped execution neither retains a handle
/// of its own nor evicts the root one.
#[test]
fn a_scoped_execution_never_inherits_a_handle_from_an_identical_compiled_lineage() {
    let model = fixture_model(PROVIDER_ID, MODEL_ID);
    let provider = continuation_provider(vec![model.clone()]);
    let provider_dyn = Arc::clone(&provider) as Arc<dyn Provider>;
    let owner = EphemeralContinuationOwner::new();
    let cold = run_on(
        Arc::clone(&provider_dyn),
        &model,
        branch(),
        compiled_context(branch(), COMPLETED_LINEAGE),
        None,
    )
    .expect("cold root attempt");
    let lineage = dispatched_fingerprint(&cold.0);

    let child = dispatch_scoped(Arc::clone(&provider_dyn), &model, child_branch())
        .expect("stateless child dispatch");
    let worker = dispatch_scoped(Arc::clone(&provider_dyn), &model, plan_worker_branch())
        .expect("stateless plan-worker dispatch");
    assert_eq!(child.output, answer(&model.id));
    assert_eq!(worker.output, answer(&model.id));
    assert_eq!(
        dispatched_fingerprint(&child),
        lineage,
        "a scoped execution can compile the identical canonical lineage, so context identity cannot be the scope boundary"
    );
    assert_eq!(dispatched_fingerprint(&worker), lineage);
    assert!(
        owner.is_empty(),
        "a child or plan-worker run must leave no handle under any execution identity"
    );

    assert!(
        retain_root_handle(&owner, provider.as_ref(), &model, &lineage),
        "the completed root turn retains its handle under the dispatched lineage"
    );
    assert_eq!(
        owner.branches(),
        vec![branch()],
        "only the canonical root lineage identity may hold a retained handle"
    );
    assert!(
        owner
            .take_if_current(
                provider.as_ref(),
                child_branch(),
                &binding_for(PROVIDER_ID, &model, &lineage)
            )
            .is_none(),
        "the root handle stays unselectable for a scoped identity that dispatched the same provider, model, and compiled lineage"
    );
    assert!(
        !owner.discard(child_branch()),
        "a scoped execution retains nothing of its own, so there is nothing to discard under that identity"
    );
    assert_eq!(
        owner.branches(),
        vec![branch()],
        "a refused scoped lookup retains nothing under the scoped identity and cannot evict the root handle"
    );

    let observed = provider.observed.lock().expect("request log");
    assert_eq!(
        observed.len(),
        3,
        "the root cold turn plus the two scoped dispatches ran"
    );
    assert!(
        observed.iter().all(|request| !request.continued),
        "a scoped dispatch sends no handle"
    );
}
