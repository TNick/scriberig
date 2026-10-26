//! Provides the scripted deterministic upstream provider used by the native
//! upstream execution fixtures.
//!
//! The provider keeps upstream stream settlement and agent-loop mechanics real
//! while making the responses, request order, and cancellation behaviour
//! deterministic for focused product tests.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rpi_ai::event_stream::create_assistant_message_event_stream;
use rpi_ai::providers::faux::faux_text;
use rpi_ai::types::{
    AssistantMessage, AssistantMessageEvent, Content, DoneReason, ErrorReason, StopReason,
};
use rpi_ai::{Context, Model, Provider, SimpleStreamOptions};

/// Host hook receiving the one-based request index before a provider request.
type RequestHook = Arc<dyn Fn(usize) + Send + Sync>;

/// One scripted provider response for one request.
#[derive(Clone)]
pub(crate) enum Step {
    /// One complete assistant message.
    Message(Box<AssistantMessage>),
    /// A request that settles only after the branch cancellation token fires.
    HoldUntilCancelled,
}

/// Deterministic upstream provider with a scripted step queue.
pub(crate) struct ScriptedProvider {
    /// Stable fixture provider identity.
    provider_id: String,
    /// Exact model row served by the fixture.
    served: Model,
    /// Steps consumed in request order.
    steps: Mutex<VecDeque<Step>>,
    /// Number of provider requests already dispatched.
    pub(crate) requests: AtomicUsize,
    /// Optional host hook receiving the one-based request index.
    hook: Mutex<Option<RequestHook>>,
}

impl ScriptedProvider {
    /// Builds a fixture provider serving one model row.
    ///
    /// # Parameters
    /// - `provider_id`: Stable fixture provider identity.
    /// - `served`: Exact model row the fixture serves.
    /// - `steps`: Steps consumed in request order.
    ///
    /// # Returns
    /// A fixture provider with the supplied script.
    pub(crate) fn new(provider_id: &str, served: Model, steps: Vec<Step>) -> Self {
        Self {
            provider_id: provider_id.to_owned(),
            served,
            steps: Mutex::new(steps.into()),
            requests: AtomicUsize::new(0),
            hook: Mutex::new(None),
        }
    }

    /// Installs a host hook that runs before each provider request.
    ///
    /// - `self`: Fixture provider whose requests are observed.
    /// - `hook`: Callback receiving the one-based request index.
    pub(crate) fn set_hook(&self, hook: impl Fn(usize) + Send + Sync + 'static) {
        if let Ok(mut current) = self.hook.lock() {
            *current = Some(Arc::new(hook));
        }
    }

    /// Returns the number of provider requests already dispatched.
    ///
    /// # Returns
    /// The fixture request count.
    pub(crate) fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    /// Streams one terminal assistant event only after cancellation fires.
    ///
    /// - `model`: Exact model selected by the upstream loop.
    /// - `signal`: Upstream cancellation signal for this request.
    ///
    /// # Returns
    /// The response event stream that settles only after cancellation.
    fn held_stream(
        model: &Model,
        signal: tokio_util::sync::CancellationToken,
    ) -> rpi_ai::AssistantMessageEventStream {
        let (mut producer, stream) = create_assistant_message_event_stream();
        let api = model.api.clone();
        let provider = model.provider.clone();
        let model_id = model.id.clone();
        tokio::spawn(async move {
            signal.cancelled().await;
            producer.push(AssistantMessageEvent::Error {
                reason: ErrorReason::Aborted,
                error: AssistantMessage::terminal(
                    api,
                    provider,
                    model_id,
                    StopReason::Aborted,
                    "provider request cancelled",
                    0,
                ),
            });
        });
        stream
    }

    /// Streams one scripted assistant message as ordered upstream events.
    ///
    /// - `message`: Scripted response for this request.
    ///
    /// # Returns
    /// The response event stream.
    fn scripted_stream(message: &AssistantMessage) -> rpi_ai::AssistantMessageEventStream {
        let (mut producer, stream) = create_assistant_message_event_stream();
        let partial = AssistantMessage {
            stop_reason: StopReason::Pending,
            ..message.clone()
        };
        producer.push(AssistantMessageEvent::Start {
            partial: Arc::new(partial),
        });
        for (index, content) in message.content.iter().enumerate() {
            match content {
                Content::Text(text) => {
                    producer.push(AssistantMessageEvent::TextStart {
                        content_index: index,
                        partial: Arc::new(message.clone()),
                    });
                    producer.push(AssistantMessageEvent::TextEnd {
                        content_index: index,
                        content: text.text.clone(),
                        partial: Arc::new(message.clone()),
                    });
                }
                Content::ToolCall(call) => {
                    producer.push(AssistantMessageEvent::ToolCallStart {
                        content_index: index,
                        partial: Arc::new(message.clone()),
                    });
                    producer.push(AssistantMessageEvent::ToolCallEnd {
                        content_index: index,
                        tool_call: call.clone(),
                        partial: Arc::new(message.clone()),
                    });
                }
                _ => {}
            }
        }
        producer.push(match message.stop_reason {
            StopReason::ToolUse | StopReason::Stop => AssistantMessageEvent::Done {
                reason: DoneReason::Stop,
                message: message.clone(),
            },
            _ => AssistantMessageEvent::Error {
                reason: ErrorReason::Error,
                error: message.clone(),
            },
        });
        stream
    }
}

#[async_trait]
impl Provider for ScriptedProvider {
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
        std::slice::from_ref(&self.served)
    }

    /// Streams the next scripted step or holds until cancellation.
    ///
    /// - `self`: Fixture provider holding the script.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `_ctx`: Provider context; the fixture answers deterministically.
    /// - `opts`: Stream options carrying the upstream cancellation signal.
    ///
    /// # Returns
    /// The response event stream for this request.
    async fn stream_simple(
        &self,
        model: &Model,
        _ctx: &Context,
        opts: &SimpleStreamOptions,
    ) -> rpi_ai::AssistantMessageEventStream {
        let index = self.requests.fetch_add(1, Ordering::SeqCst) + 1;
        if let Ok(hook) = self.hook.lock()
            && let Some(hook) = hook.as_ref()
        {
            hook(index);
        }
        let step = self
            .steps
            .lock()
            .expect("scripted provider queue")
            .pop_front()
            .unwrap_or_else(|| step(answer(model, "exhausted", "scripted provider is exhausted")));
        match step {
            Step::HoldUntilCancelled => Self::held_stream(model, opts.signal.clone()),
            Step::Message(message) => {
                let mut message = *message;
                message.api.clone_from(&model.api);
                message.provider.clone_from(&model.provider);
                message.model.clone_from(&model.id);
                Self::scripted_stream(&message)
            }
        }
    }
}

/// Builds one scripted step from a complete assistant message.
///
/// # Parameters
/// - `message`: Scripted response for one request.
///
/// # Returns
/// The queued step for that response.
pub(crate) fn step(message: AssistantMessage) -> Step {
    Step::Message(Box::new(message))
}

/// Builds one completed text response carrying an upstream response identity.
///
/// # Parameters
/// - `model`: Exact model that produced the response.
/// - `response_id`: Upstream-owned response identity marker.
/// - `text`: Assistant text content.
///
/// # Returns
/// The completed assistant message.
pub(crate) fn answer(model: &Model, response_id: &str, text: &str) -> AssistantMessage {
    AssistantMessage {
        content: vec![faux_text(text)],
        response_id: Some(format!("upstream-{response_id}")),
        stop_reason: StopReason::Stop,
        ..AssistantMessage::empty(
            model.api.clone(),
            model.provider.clone(),
            model.id.clone(),
            0,
        )
    }
}

/// Builds one tool-call response carrying upstream call identities.
///
/// # Parameters
/// - `model`: Exact model that produced the response.
/// - `response_id`: Upstream-owned response identity marker.
/// - `call_id`: Upstream-owned tool-call identity marker.
/// - `tool`: Trusted `ScribeRig` tool name selected by the provider.
/// - `arguments`: Validated tool arguments.
///
/// # Returns
/// The tool-use assistant message.
pub(crate) fn tool_use(
    model: &Model,
    response_id: &str,
    call_id: &str,
    tool: &str,
    arguments: serde_json::Value,
) -> AssistantMessage {
    AssistantMessage {
        content: vec![Content::tool_call(call_id, tool, arguments)],
        response_id: Some(format!("upstream-{response_id}")),
        stop_reason: StopReason::ToolUse,
        ..AssistantMessage::empty(
            model.api.clone(),
            model.provider.clone(),
            model.id.clone(),
            0,
        )
    }
}
