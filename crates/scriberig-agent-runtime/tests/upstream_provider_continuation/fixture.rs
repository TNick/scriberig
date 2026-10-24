//! Provides the deterministic continuation-capable provider used by the
//! upstream continuation boundary tests.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use rpi_ai::event_stream::create_assistant_message_event_stream;
use rpi_ai::provider::ProviderContinuation;
use rpi_ai::providers::faux::faux_text;
use rpi_ai::types::{Api, AssistantMessage, AssistantMessageEvent, DoneReason, StopReason};
use rpi_ai::{Context, Model, Provider, SimpleStreamOptions};
use scriberig_agent_runtime::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchInvocation, UpstreamBranchResult,
    run_upstream_branch_invocation_with_continuation,
};
use scriberig_core::EntityId;
use tokio_util::sync::CancellationToken;

/// Private provider-owned continuation state, mirroring the opaque upstream
/// contract: the host carries the handle but cannot interpret its content.
pub(super) struct FixtureContinuationState {
    /// Response identity this handle continues.
    pub(super) response_id: String,
}

/// One provider request observed by the fixture.
pub(super) struct ObservedRequest {
    /// Whether the provider received a continuation handle for this request.
    pub(super) continued: bool,
    /// Provider-owned marker read out of the supplied handle's private state.
    pub(super) continued_response_id: Option<String>,
    /// Response identity carried by the produced assistant message.
    pub(super) response_id: String,
}

/// Deterministic provider that advertises and consumes response continuation.
pub(super) struct ContinuationProvider {
    /// Stable fixture provider identity.
    provider_id: String,
    /// Exact model rows served by the fixture.
    served: Vec<Model>,
    /// Whether the provider advertises audited response continuation.
    supported: bool,
    /// Requests already dispatched through the continuation-aware seam.
    pub(super) observed: std::sync::Mutex<Vec<ObservedRequest>>,
    /// Monotonic response identity counter shared by all requests.
    responses: AtomicUsize,
}

impl ContinuationProvider {
    /// Builds a fixture provider serving the supplied model rows.
    ///
    /// # Parameters
    /// - `provider_id`: Stable fixture provider identity.
    /// - `served`: Model rows the fixture serves.
    /// - `supported`: Whether audited continuation is advertised.
    ///
    /// # Returns
    /// A fixture provider with an empty request log.
    pub(super) fn new(provider_id: &str, served: Vec<Model>, supported: bool) -> Self {
        Self {
            provider_id: provider_id.to_owned(),
            served,
            supported,
            observed: std::sync::Mutex::new(Vec::new()),
            responses: AtomicUsize::new(0),
        }
    }

    /// Records one provider request and returns its fresh response identity.
    ///
    /// # Parameters
    /// - `self`: Fixture provider holding the request log.
    /// - `continuation`: Handle supplied by the host, when any.
    ///
    /// # Returns
    /// The response identity carried by the produced message.
    fn record(&self, continuation: Option<&ProviderContinuation>) -> String {
        let response_id = format!("response-{}", self.responses.fetch_add(1, Ordering::SeqCst));
        let continued_response_id = continuation.and_then(|handle| {
            handle.with_state::<FixtureContinuationState, _>(|state| state.response_id.clone())
        });
        if let Ok(mut observed) = self.observed.lock() {
            observed.push(ObservedRequest {
                continued: continuation.is_some(),
                continued_response_id,
                response_id: response_id.clone(),
            });
        }
        response_id
    }

    /// Builds one completed assistant response for a recorded request.
    ///
    /// # Parameters
    /// - `self`: Fixture provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `response_id`: Response identity carried by the assistant message.
    ///
    /// # Returns
    /// The completed assistant message.
    pub(super) fn completed(&self, model: &Model, response_id: &str) -> AssistantMessage {
        AssistantMessage {
            response_id: Some(response_id.to_owned()),
            stop_reason: StopReason::Stop,
            content: vec![faux_text(answer(&model.id))],
            ..AssistantMessage::empty(
                model.api.clone(),
                self.provider_id.clone(),
                model.id.clone(),
                0,
            )
        }
    }

    /// Streams one completed response for a recorded request.
    ///
    /// # Parameters
    /// - `self`: Fixture provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `response_id`: Response identity carried by the assistant message.
    ///
    /// # Returns
    /// The response event stream.
    fn stream_completed(
        &self,
        model: &Model,
        response_id: &str,
    ) -> rpi_ai::AssistantMessageEventStream {
        let (mut producer, stream) = create_assistant_message_event_stream();
        let message = self.completed(model, response_id);
        let partial = AssistantMessage {
            stop_reason: StopReason::Pending,
            ..message.clone()
        };
        producer.push(AssistantMessageEvent::Start {
            partial: Arc::new(partial),
        });
        producer.push(AssistantMessageEvent::TextStart {
            content_index: 0,
            partial: Arc::new(message.clone()),
        });
        producer.push(AssistantMessageEvent::TextEnd {
            content_index: 0,
            content: answer(&model.id),
            partial: Arc::new(message.clone()),
        });
        producer.push(AssistantMessageEvent::Done {
            reason: DoneReason::Stop,
            message,
        });
        stream
    }
}

#[async_trait]
impl Provider for ContinuationProvider {
    /// Returns the fixture provider identity.
    ///
    /// # Returns
    /// Stable fixture provider identifier.
    fn id(&self) -> &str {
        &self.provider_id
    }

    /// Returns the model rows served by the fixture.
    ///
    /// # Returns
    /// Exact fixture model rows.
    fn models(&self) -> &[Model] {
        &self.served
    }

    /// Reports whether audited response continuation is advertised.
    ///
    /// # Returns
    /// The fixture's advertised continuation capability.
    fn supports_response_continuation(&self) -> bool {
        self.supported
    }

    /// Streams a request that carried no continuation handle.
    ///
    /// - `self`: Fixture provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `_ctx`: Provider context, unused by the fixture.
    /// - `_opts`: Stream options, unused by the fixture.
    ///
    /// # Returns
    /// The response event stream for a fresh response identity.
    async fn stream_simple(
        &self,
        model: &Model,
        _ctx: &Context,
        _opts: &SimpleStreamOptions,
    ) -> rpi_ai::AssistantMessageEventStream {
        let response_id = self.record(None);
        self.stream_completed(model, &response_id)
    }

    /// Extracts an opaque handle naming the response it continues.
    ///
    /// - `self`: Fixture provider holding identity and served models.
    /// - `model`: Exact model that produced the response.
    /// - `response`: Completed assistant response.
    ///
    /// # Returns
    /// A provider/model-scoped handle for a completed, supported response.
    fn continuation_from_response(
        &self,
        model: &Model,
        response: &AssistantMessage,
    ) -> Option<ProviderContinuation> {
        if !self.supported
            || matches!(
                response.stop_reason,
                StopReason::Error | StopReason::Aborted
            )
        {
            return None;
        }
        let response_id = response.response_id.clone()?;
        Some(ProviderContinuation::new(
            self.provider_id.clone(),
            model.id.clone(),
            FixtureContinuationState { response_id },
        ))
    }

    /// Streams a continuation-aware request and records the supplied handle.
    ///
    /// - `self`: Fixture provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `_ctx`: Provider context; the fixture responds deterministically.
    /// - `continuation`: Handle offered by the host after its own validation.
    /// - `_opts`: Stream options, unused by the fixture.
    /// - `_request_secret`: Protected credential, unused by the fixture.
    /// - `_callback`: Applied-settings sink, unused by the fixture.
    ///
    /// # Returns
    /// The response event stream for a fresh response identity.
    async fn stream_simple_with_continuation_and_applied_settings(
        &self,
        model: &Model,
        _ctx: &Context,
        continuation: Option<&ProviderContinuation>,
        _opts: &SimpleStreamOptions,
        _request_secret: Option<(&rpi_ai::RequestSecret, bool)>,
        _callback: rpi_ai::AppliedRequestSettingsCallback,
    ) -> rpi_ai::AssistantMessageEventStream {
        let response_id = self.record(continuation);
        self.stream_completed(model, &response_id)
    }
}

/// Builds one compiled branch invocation bound to the fixture provider.
///
/// # Parameters
/// - `provider`: Fixture provider under test.
/// - `model`: Exact model bound to the invocation.
///
/// # Returns
/// A branch invocation with no semantic tools.
pub(super) fn invocation(provider: Arc<dyn Provider>, model: Model) -> UpstreamBranchInvocation {
    let context =
        ContextCompiler::new(ContextPolicy::default()).compile(&BranchExecutionContext::new(
            "continuation-session",
            "continuation-branch",
            "continuation lifecycle proof",
            "continue the same branch",
        ));
    UpstreamBranchInvocation {
        branch: EntityId::from_bytes([91; 16]),
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
    }
}

/// Builds one fixture model row.
///
/// # Parameters
/// - `provider_id`: Provider identity owning the model.
/// - `model_id`: Model identity served by the fixture.
///
/// # Returns
/// An upstream model descriptor bound to the fixture provider.
pub(super) fn fixture_model(provider_id: &str, model_id: &str) -> Model {
    Model::new(
        model_id,
        "Continuation Fixture Model",
        Api::Other("continuation-fixture".to_owned()),
        provider_id,
        "https://provider.invalid",
    )
}

/// Runs one branch invocation on a current-thread runtime.
///
/// # Parameters
/// - `provider`: Fixture provider under test.
/// - `model`: Exact model bound to the invocation.
/// - `continuation`: Handle offered to the upstream loop, when any.
///
/// # Returns
/// The invocation outcome including any extracted handle.
///
/// # Errors
/// Returns the branch invocation failure.
pub(super) fn run(
    provider: Arc<dyn Provider>,
    model: Model,
    continuation: Option<ProviderContinuation>,
) -> Result<
    (UpstreamBranchResult, Option<ProviderContinuation>),
    scriberig_agent_runtime::upstream_invocation::UpstreamBranchFailure,
> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("continuation runtime");
    runtime.block_on(run_upstream_branch_invocation_with_continuation(
        invocation(provider, model),
        "continue the same branch",
        CancellationToken::new(),
        None,
        None,
        continuation,
    ))
}

/// Returns the deterministic answer text for one fixture model.
///
/// # Parameters
/// - `model_id`: Exact fixture model identity.
///
/// # Returns
/// The assistant text the fixture produces.
pub(super) fn answer(model_id: &str) -> String {
    format!("{model_id} answer")
}
