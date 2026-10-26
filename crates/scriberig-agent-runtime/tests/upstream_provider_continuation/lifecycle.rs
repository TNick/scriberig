//! Covers the host-owned provider continuation boundary for one branch run.

use std::sync::Arc;

use rpi_ai::{Provider, ProviderContinuation};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchError, run_upstream_branch_invocation,
};
use tokio_util::sync::CancellationToken;

use super::fixture::{ContinuationProvider, answer, fixture_model, invocation, run};

const PROVIDER_ID: &str = "continuation-fixture";
const SCOPE_REJECTION: &str = "provider continuation is outside the selected provider/model scope";

/// A completed cold response yields one opaque handle and sends no handle.
#[test]
fn a_cold_invocation_extracts_one_scoped_handle_for_the_host() {
    let model = fixture_model(PROVIDER_ID, "continuation-model");
    let provider = Arc::new(ContinuationProvider::new(
        PROVIDER_ID,
        vec![model.clone()],
        true,
    ));

    let (result, continuation) = run(
        Arc::clone(&provider) as Arc<dyn Provider>,
        model.clone(),
        None,
    )
    .expect("cold continuation invocation");
    let continuation = continuation.expect("extracted continuation");

    assert_eq!(result.output, answer(&model.id));
    assert!(continuation.is_scoped_to(PROVIDER_ID, &model.id));
    let observed = provider.observed.lock().expect("request log");
    assert_eq!(observed.len(), 1);
    assert!(
        !observed[0].continued,
        "a cold root turn must not send a continuation handle"
    );
}

/// A host-validated handle reaches the provider request and a fresh handle returns.
#[test]
fn a_host_supplied_handle_is_consumed_by_the_provider_request() {
    let model = fixture_model(PROVIDER_ID, "continuation-model");
    let provider = Arc::new(ContinuationProvider::new(
        PROVIDER_ID,
        vec![model.clone()],
        true,
    ));
    let provider_dyn = Arc::clone(&provider) as Arc<dyn Provider>;

    let (_, first) = run(Arc::clone(&provider_dyn), model.clone(), None).expect("cold invocation");
    let first = first.expect("cold extracted handle");
    let (_, second) = run(
        Arc::clone(&provider_dyn),
        model.clone(),
        Some(ProviderContinuation::clone(&first)),
    )
    .expect("continued invocation");

    let observed = provider.observed.lock().expect("request log");
    assert_eq!(observed.len(), 2);
    assert!(!observed[0].continued);
    assert!(
        observed[1].continued,
        "the validated handle must reach the provider continuation request"
    );
    assert_eq!(
        observed[1].continued_response_id.as_deref(),
        Some(observed[0].response_id.as_str()),
        "the provider must continue the exact prior response"
    );
    let second = second.expect("continued response handle");
    assert_ne!(second, first, "a continued response yields its own handle");
}

/// A handle produced for another model is rejected before any provider request.
#[test]
fn an_out_of_scope_handle_is_rejected_before_dispatch() {
    let model = fixture_model(PROVIDER_ID, "continuation-model");
    let other = fixture_model(PROVIDER_ID, "continuation-model-later");
    let provider = Arc::new(ContinuationProvider::new(
        PROVIDER_ID,
        vec![model.clone(), other.clone()],
        true,
    ));

    let foreign = provider
        .continuation_from_response(&other, &provider.completed(&other, "response-foreign"))
        .expect("foreign model handle");
    let failure = run(
        Arc::clone(&provider) as Arc<dyn Provider>,
        model,
        Some(foreign),
    )
    .expect_err("out-of-scope handle");

    assert_eq!(
        failure.error,
        UpstreamBranchError::Agent(SCOPE_REJECTION.to_owned())
    );
    assert!(
        failure.model_requests.is_empty(),
        "an out-of-scope handle must not dispatch a provider request"
    );
    assert!(
        provider.observed.lock().expect("request log").is_empty(),
        "the rejected invocation must not reach the provider"
    );
}

/// A provider without continuation support fails closed on a supplied handle.
#[test]
fn an_unsupported_provider_rejects_the_handle_before_dispatch() {
    let model = fixture_model(PROVIDER_ID, "continuation-model");
    let supported = Arc::new(ContinuationProvider::new(
        PROVIDER_ID,
        vec![model.clone()],
        true,
    ));
    let handle = supported
        .continuation_from_response(&model, &supported.completed(&model, "response-supported"))
        .expect("supported handle");
    let unsupported = Arc::new(ContinuationProvider::new(
        PROVIDER_ID,
        vec![model.clone()],
        false,
    ));

    let failure = run(unsupported as Arc<dyn Provider>, model, Some(handle))
        .expect_err("unsupported provider handle");

    assert_eq!(
        failure.error,
        UpstreamBranchError::Agent(SCOPE_REJECTION.to_owned())
    );
    assert!(
        supported.observed.lock().expect("request log").is_empty(),
        "the rejected invocation must not reach the provider"
    );
}

/// The plan-worker and one-shot entry point stays stateless across invocations.
///
/// `run_upstream_branch_invocation` is the seam used by precompiled plan-worker
/// dispatch; it neither accepts a handle nor returns one to its caller.
#[test]
fn the_precompiled_entry_point_never_carries_a_handle() {
    let model = fixture_model(PROVIDER_ID, "continuation-model");
    let provider = Arc::new(ContinuationProvider::new(
        PROVIDER_ID,
        vec![model.clone()],
        true,
    ));
    let provider_dyn = Arc::clone(&provider) as Arc<dyn Provider>;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("continuation runtime");

    for _ in 0..2 {
        let result = runtime
            .block_on(run_upstream_branch_invocation(
                invocation(Arc::clone(&provider_dyn), model.clone()),
                "continue the same branch",
                CancellationToken::new(),
                None,
            ))
            .expect("stateless precompiled invocation");
        assert_eq!(result.output, answer(&model.id));
    }

    let observed = provider.observed.lock().expect("request log");
    assert_eq!(observed.len(), 2);
    assert!(
        observed.iter().all(|request| !request.continued),
        "plan-worker dispatch must seed from canonical ScribeRig context"
    );
}
