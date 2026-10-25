//! Runs one `ScribeRig`-compiled branch invocation through the upstream agent loop.

use std::collections::HashSet;
use std::sync::Arc;

use rpi_agent::{Agent, AgentBuilder, AgentEvent, AgentTool, ToolExecutionMode};
use rpi_ai::{AssistantMessageEvent, ProviderContinuation};
use tokio_util::sync::CancellationToken;

#[path = "upstream_invocation/request_projection.rs"]
mod request_projection;
/// Stores the bounded request projections captured during one branch run.
type CapturedRequestSink = Arc<std::sync::Mutex<Vec<scriberig_session::ModelRequestProjection>>>;

mod types;
pub use types::{
    UpstreamBranchError, UpstreamBranchFailure, UpstreamBranchInvocation, UpstreamBranchResult,
};

#[path = "upstream_invocation/blocking.rs"]
mod blocking;
pub use blocking::run_upstream_branch_invocation_blocking;

#[path = "upstream_invocation/applied_settings.rs"]
mod applied_settings;
#[path = "upstream_invocation/failure.rs"]
mod failure;
use failure::{classify_agent_state, classify_loop_failure};
#[path = "upstream_invocation/provider_stream.rs"]
mod provider_stream;
use provider_stream::build_provider_stream;

/// Executes one compiled branch using upstream provider streaming and tool settlement.
///
/// The compiled context and approved tool set are transient input to a fresh
/// upstream agent. `ScribeRig` branch identity and result persistence remain
/// outside upstream state.
///
/// # Parameters
/// - `invocation`: Exact provider, model, branch context, and approved tools.
/// - `prompt`: Current user instruction for this branch invocation.
/// - `cancellation`: `ScribeRig` branch-local cancellation token.
/// - `output_callback`: Optional sink for provider-visible assistant text only.
///
/// # Returns
/// The final assistant text paired with the original `ScribeRig` branch ID.
///
/// # Errors
/// Returns an upstream loop failure, cancellation, or missing final assistant
/// message.
pub async fn run_upstream_branch_invocation(
    invocation: UpstreamBranchInvocation,
    prompt: impl Into<String>,
    cancellation: CancellationToken,
    output_callback: Option<crate::OutputCallback>,
) -> Result<UpstreamBranchResult, UpstreamBranchFailure> {
    run_upstream_branch_invocation_with_continuation(
        invocation,
        prompt,
        cancellation,
        output_callback,
        None,
        None,
    )
    .await
    .map(|(result, _)| result)
}

/// Executes a branch with an optional host-validated provider continuation.
///
/// The continuation remains in process memory and is returned separately from
/// the durable result projection.
///
/// # Parameters
/// - `invocation`: Exact provider, model, context, and approved branch tools.
/// - `prompt`: Current user instruction for this branch invocation.
/// - `cancellation`: Branch-local cancellation token.
/// - `output_callback`: Optional provider-visible text sink.
/// - `summary_callback`: Optional provider-visible reasoning-summary sink with a
///   per-invocation arrival ordinal. Raw thinking events are never forwarded.
/// - `continuation`: Optional previously validated provider continuation.
///
/// # Returns
/// Durable invocation result and an updated transient continuation.
///
/// # Errors
/// Returns a loop failure, cancellation, or missing final assistant response.
pub async fn run_upstream_branch_invocation_with_continuation(
    invocation: UpstreamBranchInvocation,
    prompt: impl Into<String>,
    cancellation: CancellationToken,
    output_callback: Option<crate::OutputCallback>,
    summary_callback: Option<crate::ReasoningSummaryCallback>,
    continuation: Option<ProviderContinuation>,
) -> Result<(UpstreamBranchResult, Option<ProviderContinuation>), UpstreamBranchFailure> {
    let built = build_agent(&invocation, continuation)?;
    let prompt = prompt.into();
    let mut events = built.agent.subscribe();
    let run = built.agent.prompt(prompt);
    tokio::pin!(run);
    let mut events_open = true;
    let mut summary_ordinal = 0_u64;
    let run_result = loop {
        tokio::select! {
            () = cancellation.cancelled() => {
                built.agent.abort();
                break Err(UpstreamBranchError::Cancelled);
            }
            result = &mut run => {
                break result.map_err(|error| classify_loop_failure(&error));
            }
            event = events.recv(), if events_open => {
                match event {
                    Ok(event) => forward_event(
                        event,
                        output_callback.as_ref(),
                        summary_callback.as_ref(),
                        &mut summary_ordinal,
                    ),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => events_open = false,
                }
            }
        }
    };
    // A cancellation deliberately stops forwarding: the host already abandoned
    // this branch and must not receive text that arrived after it did.
    if !matches!(&run_result, Err(UpstreamBranchError::Cancelled)) {
        drain_buffered_events(
            &mut events,
            output_callback.as_ref(),
            summary_callback.as_ref(),
            &mut summary_ordinal,
        );
    }
    let model_requests = built
        .captured_requests
        .lock()
        .map(|requests| requests.clone())
        .unwrap_or_default();
    let failure = |error| UpstreamBranchFailure {
        error,
        model_requests: model_requests.clone(),
    };
    run_result.map_err(failure)?;

    let state = built.agent.state();
    // Upstream owns provider transport and authorization classification. The
    // typed terminal reason it records on agent state is the only input used to
    // choose the public ScribeRig category.
    if let Some(error) = classify_agent_state(&state) {
        return Err(failure(error));
    }
    let assistant = state
        .messages
        .iter()
        .rev()
        .find_map(|message| message.as_assistant())
        .ok_or_else(|| failure(UpstreamBranchError::MissingFinalAssistant))?;
    let output = assistant
        .content
        .iter()
        .filter_map(|content| match content {
            rpi_ai::types::Content::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    let continuation = built
        .captured_continuation
        .lock()
        .ok()
        .and_then(|value| value.clone());
    Ok((
        UpstreamBranchResult {
            branch: invocation.branch,
            output,
            model_requests,
        },
        continuation,
    ))
}

/// Forwards provider-visible text from one upstream subscription event.
///
/// Assistant text deltas and reasoning-summary deltas are the only events the
/// host observes; raw thinking, tool, and lifecycle events never reach a sink.
///
/// # Parameters
/// - `event`: Event received from the branch's upstream broadcast.
/// - `output_callback`: Optional assistant text sink.
/// - `summary_callback`: Optional reasoning-summary sink with a per-invocation
///   arrival ordinal.
/// - `summary_ordinal`: Arrival ordinal advanced for each summary delta.
fn forward_event(
    event: AgentEvent,
    output_callback: Option<&crate::OutputCallback>,
    summary_callback: Option<&crate::ReasoningSummaryCallback>,
    summary_ordinal: &mut u64,
) {
    match event {
        AgentEvent::MessageUpdate {
            assistant_message_event: AssistantMessageEvent::TextDelta { delta, .. },
            ..
        } => {
            if let Some(callback) = output_callback {
                callback(&delta);
            }
        }
        AgentEvent::MessageUpdate {
            assistant_message_event: AssistantMessageEvent::ReasoningSummaryDelta { delta, .. },
            ..
        } => {
            if let Some(callback) = summary_callback {
                *summary_ordinal = summary_ordinal.saturating_add(1);
                callback(*summary_ordinal, &delta);
            }
        }
        _ => {}
    }
}

/// Forwards events still buffered when the upstream loop settles.
///
/// The prompt future and the subscription receiver are polled by the same
/// `select!`, and the upstream loop emits its closing deltas in the poll that
/// settles the run, so the loop can settle with those deltas unread. They are
/// already in the broadcast once the run resolves, so this drain is bounded.
///
/// # Parameters
/// - `events`: Subscription receiver of the settled branch run.
/// - `output_callback`: Optional assistant text sink.
/// - `summary_callback`: Optional reasoning-summary sink.
/// - `summary_ordinal`: Arrival ordinal advanced for each summary delta.
fn drain_buffered_events(
    events: &mut tokio::sync::broadcast::Receiver<AgentEvent>,
    output_callback: Option<&crate::OutputCallback>,
    summary_callback: Option<&crate::ReasoningSummaryCallback>,
    summary_ordinal: &mut u64,
) {
    loop {
        match events.try_recv() {
            Ok(event) => forward_event(event, output_callback, summary_callback, summary_ordinal),
            // A lagged receiver already dropped whole events; keep draining the
            // events that are still buffered.
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
            Err(
                tokio::sync::broadcast::error::TryRecvError::Empty
                | tokio::sync::broadcast::error::TryRecvError::Closed,
            ) => return,
        }
    }
}

/// Builds a fresh upstream agent after validating branch tool ownership.
///
/// # Parameters
/// - `invocation`: Exact provider, model, branch context, and approved tools.
///
/// # Returns
/// The branch-local agent and its safe request projection sink.
///
/// # Errors
/// Returns a scoped-tool or upstream agent-construction failure.
struct BuiltAgent {
    /// Fresh upstream agent configured for one branch invocation.
    agent: Agent,
    /// Captured safe request projections from provider dispatch.
    captured_requests: CapturedRequestSink,
    /// Continuation returned by the upstream loop, when supported.
    captured_continuation: Arc<std::sync::Mutex<Option<ProviderContinuation>>>,
}

/// Builds a fresh upstream agent after validating branch tool ownership.
///
/// # Parameters
/// - `invocation`: Exact provider, model, context, and approved tools.
/// - `continuation`: Optional host-validated provider continuation.
///
/// # Returns
/// Branch-local agent and the transient request and continuation capture sinks.
///
/// # Errors
/// Returns a scoped-tool or upstream agent-construction failure.
fn build_agent(
    invocation: &UpstreamBranchInvocation,
    continuation: Option<ProviderContinuation>,
) -> Result<BuiltAgent, UpstreamBranchFailure> {
    let mut tool_names = HashSet::with_capacity(invocation.tools.len());
    if invocation.tools.iter().any(|tool| {
        let name = tool.schema().name.as_str();
        tool.branch() != invocation.branch
            || name.trim().is_empty()
            || !tool_names.insert(name.to_owned())
    }) {
        return Err(UpstreamBranchFailure {
            error: UpstreamBranchError::InvalidToolSet,
            model_requests: Vec::new(),
        });
    }

    let provider = Arc::clone(&invocation.provider);
    if let Some(continuation) = &continuation
        && (!provider.supports_response_continuation()
            || !continuation.is_scoped_to(provider.id(), &invocation.model.id))
    {
        return Err(UpstreamBranchFailure {
            error: UpstreamBranchError::Agent(
                "provider continuation is outside the selected provider/model scope".to_owned(),
            ),
            model_requests: Vec::new(),
        });
    }
    let continuation_enabled = provider.supports_response_continuation();
    let model_requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured_requests = Arc::clone(&model_requests);
    let stream = build_provider_stream(invocation, captured_requests);
    let tools: Vec<Arc<dyn AgentTool>> = invocation
        .tools
        .iter()
        .cloned()
        .map(|tool| tool as Arc<dyn AgentTool>)
        .collect();
    let captured_continuation = Arc::new(std::sync::Mutex::new(None));
    let extractor_provider = Arc::clone(&provider);
    let extractor_model = invocation.model.clone();
    let extractor: rpi_agent::ContinuationExtractor =
        Arc::new(move |_requested_model, response| {
            if !continuation_enabled
                || matches!(
                    response.stop_reason,
                    rpi_ai::types::StopReason::Error | rpi_ai::types::StopReason::Aborted
                )
            {
                return None;
            }
            extractor_provider
                .continuation_from_response(&extractor_model, response)
                .filter(|continuation| {
                    continuation.is_scoped_to(extractor_provider.id(), &extractor_model.id)
                })
        });
    let continuation_sink = Arc::clone(&captured_continuation);
    let plain_provider = Arc::clone(&provider);
    let plain_model = invocation.model.clone();
    let plain_secret = invocation.request_secret.clone();
    let plain_bearer = invocation.bearer;
    let plain_stream = rpi_agent::stream_fn(move |_requested, context, options| {
        rpi_ai::stream_provider_cancellable(
            Arc::clone(&plain_provider),
            plain_model.clone(),
            context.clone(),
            options.clone(),
            plain_secret.clone().map(|secret| (secret, plain_bearer)),
            None,
            None,
            false,
            tokio_util::sync::CancellationToken::new(),
        )
    });
    let builder = AgentBuilder::new()
        .model(invocation.model.clone())
        .system_prompt(invocation.context.provider_context.clone())
        .thinking_level(invocation.thinking_level)
        .tools(tools)
        .tool_execution(ToolExecutionMode::Sequential)
        .stream_fn(plain_stream)
        .cancellable_provider_continuation(stream, extractor)
        .on_provider_continuation(Arc::new(move |continuation| {
            if let Ok(mut current) = continuation_sink.lock() {
                *current = continuation;
            }
        }));
    let built = builder.build();
    let agent: Agent = built.map_err(|error| UpstreamBranchFailure {
        error: UpstreamBranchError::Agent(error.to_string()),
        model_requests: Vec::new(),
    })?;
    agent.set_provider_continuation(continuation);
    Ok(BuiltAgent {
        agent,
        captured_requests: model_requests,
        captured_continuation,
    })
}
