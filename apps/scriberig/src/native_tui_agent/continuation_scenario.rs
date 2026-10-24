//! Supplies a continuation-capable deterministic provider for root continuation
//! lifecycle coverage.
//!
//! `ScribeRig`'s root runner owns the ephemeral handle cache; this fixture only
//! records whether a validated handle reached the provider request so the
//! ownership boundary stays observable from the app side. The opaque response
//! marker inside the handle must never reach durable state.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rpi_ai::event_stream::{
    AssistantMessageEventStreamProducer, create_assistant_message_event_stream,
};
use rpi_ai::provider::ProviderContinuation;
use rpi_ai::providers::faux::faux_text;
use rpi_ai::types::{AssistantMessage, AssistantMessageEvent, DoneReason, ErrorReason, StopReason};
use rpi_ai::{
    AppliedRequestSettingsCallback, Context, Model, Provider, RequestSecret, SimpleStreamOptions,
};

/// Terminal provider behavior for one scenario request.
pub(crate) enum ScenarioOutcome {
    /// The request completes and yields a continuable response.
    Completed,
    /// The request fails as an ordinary provider fault.
    Failed,
    /// The request is aborted by the provider.
    Aborted,
}

/// Private provider-owned continuation state.
struct ScenarioContinuationState {
    /// Opaque response marker this handle continues.
    response_marker: String,
}

/// One provider request recorded by the scenario provider.
struct ScenarioRequest {
    /// Whether the provider received a continuation handle.
    continued: bool,
    /// Opaque marker read out of the supplied handle's private state.
    continued_marker: Option<String>,
    /// Opaque marker carried by the produced response.
    response_marker: String,
}

/// Deterministic provider that advertises and consumes response continuation.
pub(crate) struct ContinuationScenarioProvider {
    /// Stable fixture provider identity.
    provider_id: String,
    /// Exact model rows served by the fixture.
    served: Vec<Model>,
    /// Requests already dispatched through the continuation-aware seam.
    observed: Mutex<Vec<ScenarioRequest>>,
    /// Monotonic response counter shared by all requests.
    responses: AtomicUsize,
    /// Terminal behavior for each successive request, defaulting to completed.
    outcomes: Mutex<Vec<ScenarioOutcome>>,
}

impl ContinuationScenarioProvider {
    /// Builds a scenario provider serving the supplied model rows.
    ///
    /// # Parameters
    /// - `provider_id`: Stable fixture provider identity.
    /// - `served`: Model rows the fixture serves.
    ///
    /// # Returns
    /// A scenario provider that completes every request.
    pub(crate) fn new(provider_id: &str, served: Vec<Model>) -> Self {
        Self {
            provider_id: provider_id.to_owned(),
            served,
            observed: Mutex::new(Vec::new()),
            responses: AtomicUsize::new(0),
            outcomes: Mutex::new(Vec::new()),
        }
    }

    /// Queues terminal outcomes for the next requests.
    ///
    /// - `self`: Scenario provider holding the outcome queue.
    /// - `outcomes`: Outcomes consumed in request order.
    pub(crate) fn queue_outcomes(&self, outcomes: Vec<ScenarioOutcome>) {
        if let Ok(mut queued) = self.outcomes.lock() {
            *queued = outcomes;
        }
    }

    /// Returns `(continued, supplied marker)` for every recorded request.
    ///
    /// - `self`: Scenario provider holding the request log.
    ///
    /// # Returns
    /// The recorded request outcomes in request order.
    pub(crate) fn requests(&self) -> Vec<(bool, Option<String>)> {
        let Ok(observed) = self.observed.lock() else {
            return Vec::new();
        };
        observed
            .iter()
            .map(|request| (request.continued, request.continued_marker.clone()))
            .collect()
    }

    /// Returns the opaque marker of the response produced by request `index`.
    ///
    /// # Parameters
    /// - `self`: Scenario provider holding the request log.
    /// - `index`: Zero-based request position.
    ///
    /// # Returns
    /// The response marker, when that request was recorded.
    pub(crate) fn response_marker(&self, index: usize) -> Option<String> {
        self.observed.lock().ok().and_then(|observed| {
            observed
                .get(index)
                .map(|request| request.response_marker.clone())
        })
    }

    /// Records one request and returns its fresh opaque response marker.
    ///
    /// # Parameters
    /// - `self`: Scenario provider holding the request log.
    /// - `continuation`: Handle supplied by the host, when any.
    ///
    /// # Returns
    /// The marker carried by the produced response.
    fn record(&self, continuation: Option<&ProviderContinuation>) -> String {
        let index = self.responses.fetch_add(1, Ordering::SeqCst);
        let marker = format!("opaque-continuation-{index}");
        let continued_marker = continuation.and_then(|handle| {
            handle.with_state::<ScenarioContinuationState, _>(|state| state.response_marker.clone())
        });
        if let Ok(mut observed) = self.observed.lock() {
            observed.push(ScenarioRequest {
                continued: continuation.is_some(),
                continued_marker,
                response_marker: marker.clone(),
            });
        }
        marker
    }

    /// Consumes the next queued outcome.
    ///
    /// - `self`: Scenario provider holding the outcome queue.
    ///
    /// # Returns
    /// The queued outcome, defaulting to a completed response.
    fn next_outcome(&self) -> ScenarioOutcome {
        let Ok(mut queued) = self.outcomes.lock() else {
            return ScenarioOutcome::Completed;
        };
        if queued.is_empty() {
            return ScenarioOutcome::Completed;
        }
        // The consumed slot resets so later requests complete normally.
        std::mem::replace(&mut queued[0], ScenarioOutcome::Completed)
    }

    /// Streams the queued terminal outcome for one request.
    ///
    /// # Parameters
    /// - `self`: Scenario provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `marker`: Opaque response marker for the produced response.
    ///
    /// # Returns
    /// The response event stream.
    fn stream_outcome(&self, model: &Model, marker: &str) -> rpi_ai::AssistantMessageEventStream {
        let (mut producer, stream) = create_assistant_message_event_stream();
        match self.next_outcome() {
            ScenarioOutcome::Failed => {
                producer.push(AssistantMessageEvent::Error {
                    reason: ErrorReason::Error,
                    error: AssistantMessage::terminal(
                        model.api.clone(),
                        self.provider_id.clone(),
                        model.id.clone(),
                        StopReason::Error,
                        "scenario provider failure",
                        0,
                    ),
                });
            }
            ScenarioOutcome::Aborted => {
                producer.push(AssistantMessageEvent::Error {
                    reason: ErrorReason::Aborted,
                    error: AssistantMessage::terminal(
                        model.api.clone(),
                        self.provider_id.clone(),
                        model.id.clone(),
                        StopReason::Aborted,
                        "scenario provider aborted",
                        0,
                    ),
                });
            }
            ScenarioOutcome::Completed => self.stream_completed(model, marker, &mut producer),
        }
        stream
    }

    /// Emits one completed assistant response carrying the opaque marker.
    ///
    /// # Parameters
    /// - `self`: Scenario provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `marker`: Opaque response marker carried for continuation.
    /// - `producer`: Event stream producer to fill.
    fn stream_completed(
        &self,
        model: &Model,
        marker: &str,
        producer: &mut AssistantMessageEventStreamProducer,
    ) {
        let text = format!("{} answer", model.id);
        let message = AssistantMessage {
            response_id: Some(marker.to_owned()),
            stop_reason: StopReason::Stop,
            content: vec![faux_text(text.clone())],
            ..AssistantMessage::empty(
                model.api.clone(),
                self.provider_id.clone(),
                model.id.clone(),
                0,
            )
        };
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
            content: text,
            partial: Arc::new(message.clone()),
        });
        producer.push(AssistantMessageEvent::Done {
            reason: DoneReason::Stop,
            message,
        });
    }
}

#[async_trait]
impl Provider for ContinuationScenarioProvider {
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

    /// Advertises audited response continuation.
    ///
    /// # Returns
    /// `true` because this fixture implements the continuation seam.
    fn supports_response_continuation(&self) -> bool {
        true
    }

    /// Streams a request that carried no continuation handle.
    ///
    /// - `self`: Scenario provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `_ctx`: Provider context, unused by the fixture.
    /// - `_opts`: Stream options, unused by the fixture.
    ///
    /// # Returns
    /// The response event stream.
    async fn stream_simple(
        &self,
        model: &Model,
        _ctx: &Context,
        _opts: &SimpleStreamOptions,
    ) -> rpi_ai::AssistantMessageEventStream {
        let marker = self.record(None);
        self.stream_outcome(model, &marker)
    }

    /// Extracts an opaque handle naming the response it continues.
    ///
    /// - `self`: Scenario provider holding identity and served models.
    /// - `model`: Exact model that produced the response.
    /// - `response`: Completed assistant response.
    ///
    /// # Returns
    /// A provider/model-scoped handle for a completed response.
    fn continuation_from_response(
        &self,
        model: &Model,
        response: &AssistantMessage,
    ) -> Option<ProviderContinuation> {
        if matches!(
            response.stop_reason,
            StopReason::Error | StopReason::Aborted
        ) {
            return None;
        }
        let marker = response.response_id.clone()?;
        Some(ProviderContinuation::new(
            self.provider_id.clone(),
            model.id.clone(),
            ScenarioContinuationState {
                response_marker: marker,
            },
        ))
    }

    /// Streams a continuation-aware request and records the supplied handle.
    ///
    /// - `self`: Scenario provider holding identity and served models.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `_ctx`: Provider context; the fixture responds deterministically.
    /// - `continuation`: Handle offered by the host after its own validation.
    /// - `_opts`: Stream options, unused by the fixture.
    /// - `_request_secret`: Protected credential, unused by the fixture.
    /// - `_callback`: Applied-settings sink, unused by the fixture.
    ///
    /// # Returns
    /// The response event stream.
    async fn stream_simple_with_continuation_and_applied_settings(
        &self,
        model: &Model,
        _ctx: &Context,
        continuation: Option<&ProviderContinuation>,
        _opts: &SimpleStreamOptions,
        _request_secret: Option<(&RequestSecret, bool)>,
        _callback: AppliedRequestSettingsCallback,
    ) -> rpi_ai::AssistantMessageEventStream {
        let marker = self.record(continuation);
        self.stream_outcome(model, &marker)
    }
}
