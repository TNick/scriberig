//! Proves every streamed assistant delta reaches the host text observer.
//!
//! The one-shot CLI and the TUI render live provider text through the
//! `OutputCallback` handed to `run_upstream_branch_invocation`. The pinned faux
//! provider can emit its last deltas and settle the loop inside one runtime
//! poll, so the invocation's subscription loop is free to settle while those
//! deltas are still buffered in the upstream event broadcast. These runs pin
//! that a settled loop still forwards every buffered delta: the product
//! `srig agent` streamed `agent output:` line, and the fail-closed plugin gate
//! that asserts that line is absent when native code is configured, both depend
//! on the streamed text arriving exactly once and in order.

use std::sync::{Arc, Mutex};

use rpi_ai::providers::faux::{FauxProvider, FauxScript, FauxStep, faux_assistant_message};
use rpi_ai::{
    Api, AppliedRequestSettingsCallback, AssistantMessageEventStream, Context, Model, Provider,
    RequestSecret, SimpleStreamOptions, StopReason,
};
use rpi_model_runtime::{
    ProviderAuthenticationOutcome, ProviderLifecycleEvent, ProviderLifecycleEvents,
};
use scriberig_agent_runtime::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};
use scriberig_agent_runtime::upstream_invocation::{
    ProviderLifecycleContext, UpstreamBranchInvocation, run_upstream_branch_invocation,
};
use scriberig_core::EntityId;
use tokio_util::sync::CancellationToken;

/// Assistant text the faux provider streams as several deltas per settlement.
const STREAMED_TEXT: &str =
    "No provider endpoint configured. The deterministic fallback answers instead.";

/// Number of consecutive settlements checked for a lost delta. The defect is a
/// settlement race, so repeated settlements are what make the regression fail.
const SETTLEMENTS: usize = 96;

/// Builds the deterministic model row for one settlement.
///
/// # Returns
/// An upstream model descriptor matching the product Echo fallback shape.
fn streamed_model() -> Model {
    Model::new(
        "echo-stream",
        "ScribeRig Echo",
        Api::Faux,
        "scriberig-echo",
        "local://echo",
    )
}

/// Builds one no-tool branch invocation scripted with `STREAMED_TEXT`.
///
/// # Returns
/// The invocation bound to the deterministic fallback provider.
fn invocation() -> UpstreamBranchInvocation {
    let model = streamed_model();
    let provider: Arc<dyn Provider> = FauxProvider::with_models(
        FauxScript::new().with_text(STREAMED_TEXT),
        vec![model.clone()],
    );
    let context =
        ContextCompiler::new(ContextPolicy::default()).compile(&BranchExecutionContext::new(
            "stream-session",
            "stream-branch",
            "stream settlement proof",
            "Reply with OK",
        ));
    UpstreamBranchInvocation {
        branch: EntityId::from_bytes([92; 16]),
        provider,
        model,
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: None,
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context,
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        provider_lifecycle: None,
    }
}

/// Provider fixture that emits a response identifier used as opaque continuation state.
struct ContinuationProvider {
    /// The sole model served by this test provider.
    model: Model,
}

/// Builds a faux provider carrying the test's opaque response marker.
///
/// # Parameters
/// - `model`: Model whose response identity will be tested.
///
/// # Returns
/// A deterministic provider scripted with continuation material as its response ID.
fn response_provider(model: &Model) -> Arc<FauxProvider> {
    let mut response = faux_assistant_message("OK", StopReason::Stop);
    response.response_id = Some("opaque-continuation-runtime-test".to_owned());
    let script = FauxScript::new();
    script.append_responses(vec![FauxStep::message(response)]);
    FauxProvider::with_models(script, vec![model.clone()])
}

#[async_trait::async_trait]
impl Provider for ContinuationProvider {
    #[expect(
        clippy::misnamed_getters,
        reason = "Provider::id identifies the provider, stored on Model::provider; Model::id is a separate model identity."
    )]
    /// Returns this fixture's provider identity.
    ///
    /// # Returns
    /// The provider identifier advertised by the fixture model.
    fn id(&self) -> &str {
        &self.model.provider
    }

    /// Returns the one model served by this fixture.
    ///
    /// # Returns
    /// The fixture model as a single-entry provider catalog.
    fn models(&self) -> &[Model] {
        std::slice::from_ref(&self.model)
    }

    /// Reports that response identifiers may be continuation material.
    ///
    /// # Returns
    /// `true` because the fixture response ID is opaque continuation state.
    fn supports_response_continuation(&self) -> bool {
        true
    }

    /// Streams the fixture response through upstream's deterministic faux provider.
    ///
    /// # Parameters
    /// - `model`: Exact model selected by the upstream loop.
    /// - `context`: Provider request context.
    /// - `options`: Non-secret request options.
    ///
    /// # Returns
    /// The faux provider response stream.
    async fn stream_simple(
        &self,
        model: &Model,
        context: &Context,
        options: &SimpleStreamOptions,
    ) -> AssistantMessageEventStream {
        response_provider(&self.model)
            .stream_simple(model, context, options)
            .await
    }

    /// Streams through the audited continuation request seam.
    ///
    /// # Parameters
    /// - `model`: Exact model selected by the upstream loop.
    /// - `context`: Provider request context.
    /// - `_continuation`: Optional opaque continuation, unused by this fixture.
    /// - `options`: Non-secret request options.
    /// - `_request_secret`: Protected credential, unused by this fixture.
    /// - `_callback`: Applied-settings sink, unused by this fixture.
    ///
    /// # Returns
    /// The faux provider response stream.
    async fn stream_simple_with_continuation_and_applied_settings(
        &self,
        model: &Model,
        context: &Context,
        _continuation: Option<&rpi_ai::ProviderContinuation>,
        options: &SimpleStreamOptions,
        _request_secret: Option<(&RequestSecret, bool)>,
        _callback: AppliedRequestSettingsCallback,
    ) -> AssistantMessageEventStream {
        response_provider(&self.model)
            .stream_simple(model, context, options)
            .await
    }
}

/// Settles one branch invocation and collects the streamed fragments.
///
/// # Returns
/// The fragments the output observer received in arrival order and the durable
/// final assistant text.
///
/// # Panics
/// Panics when the invocation executor cannot be built or the run fails.
fn settle() -> (Vec<String>, String) {
    let invocation = invocation();
    let fragments = Arc::new(Mutex::new(Vec::new()));
    let observer = {
        let fragments = Arc::clone(&fragments);
        Arc::new(move |text: &str| {
            fragments
                .lock()
                .expect("fragment sink")
                .push(text.to_owned());
        }) as Arc<dyn Fn(&str) + Send + Sync>
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("stream settlement runtime");
    let result = runtime
        .block_on(run_upstream_branch_invocation(
            invocation,
            "Reply with OK",
            CancellationToken::new(),
            Some(observer),
        ))
        .expect("stream settlement run");
    let fragments = fragments
        .lock()
        .expect("fragment sink")
        .iter()
        .cloned()
        .collect();
    (fragments, result.output)
}

/// Verifies each settlement delivers the complete streamed text exactly once.
#[test]
fn every_settlement_forwards_each_streamed_delta() {
    for settlement in 0..SETTLEMENTS {
        let (fragments, output) = settle();
        assert_eq!(output, STREAMED_TEXT, "settlement {settlement} output");
        assert!(
            fragments.len() > 1,
            "settlement {settlement} streamed {fragments:?} instead of several deltas"
        );
        assert_eq!(
            fragments.concat(),
            STREAMED_TEXT,
            "settlement {settlement} lost or reordered a streamed delta: {fragments:?}"
        );
    }
}

/// Keeps continuation-capable provider response identifiers out of provenance.
#[test]
fn continuation_response_identifiers_are_not_durable_request_provenance() {
    let mut invocation = invocation();
    invocation.provider = Arc::new(ContinuationProvider {
        model: invocation.model.clone(),
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("continuation identity runtime");
    let result = runtime
        .block_on(run_upstream_branch_invocation(
            invocation,
            "Reply with OK",
            CancellationToken::new(),
            None,
        ))
        .expect("continuation-capable provider invocation");
    assert_eq!(result.model_requests.len(), 1);
    assert_eq!(result.model_requests[0].provider_response_id, None);
}

/// Forwards upstream-authored auth evidence with the invocation catalog revision.
#[test]
fn provider_authentication_evidence_is_generation_bound_and_secret_free() {
    let mut invocation = invocation();
    let mut response = faux_assistant_message("OK", StopReason::Stop);
    response.response_id = Some("response-safe-123".to_owned());
    let script = FauxScript::new();
    script.append_responses(vec![FauxStep::message(response)]);
    invocation.provider = FauxProvider::with_models(script, vec![invocation.model.clone()]);
    let events = ProviderLifecycleEvents::new();
    let observed = events.subscribe();
    invocation.provider_lifecycle = Some(ProviderLifecycleContext {
        events,
        catalog_revision: 17,
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("auth lifecycle runtime");
    let result = runtime
        .block_on(run_upstream_branch_invocation(
            invocation,
            "Reply with OK",
            CancellationToken::new(),
            None,
        ))
        .expect("successful provider request");
    let request = result
        .model_requests
        .first()
        .expect("captured provider request");
    assert_eq!(request.catalog_revision, Some(17));
    assert_eq!(
        request.upstream_runtime_revision.as_deref(),
        Some(scriberig_agent_runtime::UPSTREAM_MODEL_RUNTIME_REVISION)
    );
    assert_eq!(
        request.provider_response_id.as_deref(),
        Some("response-safe-123")
    );
    let event = observed.recv().expect("provider auth event");
    assert_eq!(
        event,
        ProviderLifecycleEvent::AuthenticationResult(
            rpi_model_runtime::ProviderAuthenticationObservation {
                provider_id: "scriberig-echo".to_owned(),
                model_id: "echo-stream".to_owned(),
                catalog_revision: 17,
                outcome: ProviderAuthenticationOutcome::Accepted,
            }
        )
    );
}
