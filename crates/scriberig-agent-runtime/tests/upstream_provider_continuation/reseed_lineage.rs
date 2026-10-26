//! Pins the compiled-lineage half of the host reseed decision.
//!
//! `owner` and `owner_scope` drive the reseed decision with fixture values. This
//! case and `reseed_provider` drive the same decision with the identity values a
//! real attempt produces: the compiled-context fingerprint the invocation seam
//! captures in durable request provenance, and provider scope taken from distinct
//! fixture providers. Each case ends at an executed provider request, so a reseed
//! is proven as a cold canonical-context dispatch rather than only as a rejected
//! owner lookup. The attempt helpers here are shared by `reseed_provider` and
//! `execution_scope`.

use std::sync::Arc;

use rpi_ai::{Model, Provider, ProviderContinuation};
use scriberig_agent_runtime::context::{
    BranchExecutionContext, CompiledContext, ContextActivity, ContextCompiler, ContextPolicy,
};
use scriberig_agent_runtime::continuation::{
    CompletedContinuation, ContinuationBinding, ContinuationOutcome, EphemeralContinuationOwner,
};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchFailure, UpstreamBranchInvocation, UpstreamBranchResult,
    run_upstream_branch_invocation_with_continuation,
};
use scriberig_core::EntityId;
use tokio_util::sync::CancellationToken;

use super::fixture::{answer, fixture_model};
use super::owner::{MODEL_ID, PROVIDER_ID, branch, continuation_provider, handle_for};

/// Host connection/profile identity named by every binding in these cases.
pub(super) const PROFILE: &str = "host-profile";
/// Completed lineage text the retained handle was produced under.
pub(super) const COMPLETED_LINEAGE: &str = "first completed turn";
/// Divergent lineage text the next attempt compiles for the same branch.
pub(super) const DIVERGENT_LINEAGE: &str = "second completed turn";
/// Private provider state a provider-scoped handle carries.
pub(super) const HANDLE_MARKER: &str = "opaque-provider-handle";

/// Compiles one context whose canonical lineage contains `lineage`.
///
/// # Parameters
/// - `branch`: Branch identity the context is compiled for.
/// - `lineage`: Completed-turn text carried in the compiled lineage.
///
/// # Returns
/// The compiled canonical context one attempt dispatches.
pub(super) fn compiled_context(branch: EntityId, lineage: &str) -> CompiledContext {
    ContextCompiler::new(ContextPolicy::default()).compile(
        &BranchExecutionContext::new(
            "continuation-session",
            branch.to_string(),
            "continuation reseed proof",
            "continue the same branch",
        )
        .with_activity(ContextActivity::new(0, "assistant_output", lineage)),
    )
}

/// Builds one invocation for `branch` that dispatches `context`.
///
/// # Parameters
/// - `provider`: Fixture provider selected for the attempt.
/// - `model`: Exact model bound to the attempt.
/// - `branch`: Branch identity the attempt runs as.
/// - `context`: Compiled canonical context the attempt dispatches.
///
/// # Returns
/// A tool-free branch invocation for one execution identity.
pub(super) fn invocation_on(
    provider: Arc<dyn Provider>,
    model: &Model,
    branch: EntityId,
    context: CompiledContext,
) -> UpstreamBranchInvocation {
    UpstreamBranchInvocation {
        branch,
        provider,
        model: model.clone(),
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: Some(PROFILE.to_owned()),
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context,
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        provider_lifecycle: None,
    }
}

/// Runs one branch attempt through the continuation-aware invocation seam.
///
/// # Parameters
/// - `provider`: Fixture provider selected for the attempt.
/// - `model`: Exact model bound to the attempt.
/// - `branch`: Branch identity the attempt runs as.
/// - `context`: Compiled canonical context the attempt dispatches.
/// - `handle`: Handle the host offered after its own validation, when any.
///
/// # Returns
/// The executed result plus any extracted handle, or the invocation failure.
///
/// # Errors
/// Returns the branch invocation failure.
pub(super) fn run_on(
    provider: Arc<dyn Provider>,
    model: &Model,
    branch: EntityId,
    context: CompiledContext,
    handle: Option<ProviderContinuation>,
) -> Result<(UpstreamBranchResult, Option<ProviderContinuation>), UpstreamBranchFailure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("continuation runtime");
    runtime.block_on(run_upstream_branch_invocation_with_continuation(
        invocation_on(provider, model, branch, context),
        "continue the same branch",
        CancellationToken::new(),
        None,
        None,
        handle,
    ))
}

/// Returns the compiled-context fingerprint the invocation seam captured.
///
/// # Parameters
/// - `result`: Executed result carrying durable request provenance.
///
/// # Returns
/// The runtime-computed fingerprint of the compiled context actually dispatched.
pub(super) fn dispatched_fingerprint(result: &UpstreamBranchResult) -> String {
    result
        .model_requests
        .iter()
        .rev()
        .find_map(|request| request.context_fingerprint.clone())
        .expect("a dispatched request carries a compiled-context fingerprint")
}

/// Builds the binding one attempt on the named identity records.
///
/// # Parameters
/// - `provider_id`: Upstream provider identity selected for the attempt.
/// - `model`: Exact model row selected for the attempt.
/// - `lineage`: Compiled-context fingerprint of the completed lineage.
///
/// # Returns
/// The binding the owner revalidates a retained handle against.
pub(super) fn binding_for(provider_id: &str, model: &Model, lineage: &str) -> ContinuationBinding {
    ContinuationBinding::new(
        provider_id,
        model.clone(),
        PROFILE,
        rpi_ai::ThinkingLevel::Off,
        lineage,
    )
}

/// Retains one provider-scoped handle for the canonical root lineage.
///
/// # Parameters
/// - `owner`: Owner under test.
/// - `provider`: Provider that completed the root turn.
/// - `model`: Exact model the root turn used.
/// - `lineage`: Compiled-context fingerprint of that completed turn.
///
/// # Returns
/// Whether the owner retained the handle.
pub(super) fn retain_root_handle(
    owner: &EphemeralContinuationOwner,
    provider: &dyn Provider,
    model: &Model,
    lineage: &str,
) -> bool {
    owner.settle(
        provider,
        branch(),
        ContinuationOutcome::Completed(Box::new(CompletedContinuation {
            binding: binding_for(PROVIDER_ID, model, lineage),
            handle: handle_for(HANDLE_MARKER, MODEL_ID),
        })),
    )
}

/// A changed compiled lineage reseeds the next attempt and drops the stale handle.
///
/// Both lineage identities are the fingerprints the invocation seam captured in
/// durable provenance, so divergence is proven against real compiled context
/// rather than a fixture string, and the reseed is proven by the cold provider
/// request it actually dispatches.
#[test]
fn a_changed_compiled_lineage_reseeds_and_drops_the_stale_handle() {
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
    let completed_lineage = dispatched_fingerprint(&cold.0);
    assert!(
        owner.settle(
            provider.as_ref(),
            branch(),
            ContinuationOutcome::Completed(Box::new(CompletedContinuation {
                binding: binding_for(PROVIDER_ID, &model, &completed_lineage),
                handle: cold.1.expect("extracted handle"),
            })),
        ),
        "the completed turn retains its handle under the lineage that produced it"
    );

    // The host names the next attempt's lineage before dispatching it. The probe
    // runs on a separate provider instance so the branch's own request log stays
    // scoped to the two attempts under test.
    let probe = continuation_provider(vec![model.clone()]);
    let probe_run = run_on(
        Arc::clone(&probe) as Arc<dyn Provider>,
        &model,
        branch(),
        compiled_context(branch(), DIVERGENT_LINEAGE),
        None,
    )
    .expect("next lineage probe");
    let next_lineage = dispatched_fingerprint(&probe_run.0);
    assert_ne!(
        completed_lineage, next_lineage,
        "a changed compiled lineage must change the captured fingerprint"
    );

    let candidate = owner.take_if_current(
        provider.as_ref(),
        branch(),
        &binding_for(PROVIDER_ID, &model, &next_lineage),
    );
    assert!(
        candidate.is_none(),
        "a changed compiled lineage must reseed from canonical context"
    );
    assert!(
        owner.is_empty(),
        "the divergent attempt must drop the stale handle instead of keeping it for later"
    );

    let reseeded = run_on(
        provider_dyn,
        &model,
        branch(),
        compiled_context(branch(), DIVERGENT_LINEAGE),
        candidate,
    )
    .expect("reseeded root attempt");
    assert_eq!(reseeded.0.output, answer(&model.id));
    assert_eq!(
        dispatched_fingerprint(&reseeded.0),
        next_lineage,
        "the reseeded attempt dispatches the new canonical compiled context"
    );
    let observed = provider.observed.lock().expect("request log");
    assert_eq!(
        observed.len(),
        2,
        "only the completed and reseeded attempts reached this provider"
    );
    assert!(
        observed.iter().all(|request| !request.continued),
        "a divergent lineage must send no provider handle"
    );
}
