//! Relays safe provider lifecycle and assistant events from the upstream agent loop.

use rpi_agent::{Agent, AgentEvent};
use rpi_ai::AssistantMessageEvent;
use tokio_util::sync::CancellationToken;

use super::failure::classify_loop_failure;
use super::types::ProviderLifecycleContext;
use super::types::UpstreamBranchError;

/// Runs one provider turn while relaying only safe visible event streams.
///
/// # Parameters
/// - `agent`: Fresh upstream agent for this branch invocation.
/// - `prompt`: Current user instruction.
/// - `cancellation`: Branch-local cancellation token.
/// - `output_callback`: Optional assistant text sink.
/// - `summary_callback`: Optional reasoning summary sink.
/// - `provider_lifecycle`: Optional safe provider lifecycle observer.
///
/// # Returns
/// Success after the upstream run completes and buffered visible events are drained.
///
/// # Errors
/// Returns the classified upstream failure or branch cancellation.
pub(super) async fn run_agent_and_forward(
    agent: &Agent,
    prompt: String,
    cancellation: &CancellationToken,
    output_callback: Option<&crate::OutputCallback>,
    summary_callback: Option<&crate::ReasoningSummaryCallback>,
    provider_lifecycle: Option<&ProviderLifecycleContext>,
) -> Result<(), UpstreamBranchError> {
    let mut events = agent.subscribe();
    let run = agent.prompt(prompt);
    tokio::pin!(run);
    let mut events_open = true;
    let mut summary_ordinal = 0_u64;
    let run_result = loop {
        tokio::select! {
            () = cancellation.cancelled() => {
                agent.abort();
                break Err(UpstreamBranchError::Cancelled);
            }
            result = &mut run => {
                break result.map_err(|error| classify_loop_failure(&error));
            }
            event = events.recv(), if events_open => {
                match event {
                    Ok(event) => forward_event(
                        event,
                        output_callback,
                        summary_callback,
                        &mut summary_ordinal,
                        provider_lifecycle,
                    ),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => events_open = false,
                }
            }
        }
    };
    if !matches!(&run_result, Err(UpstreamBranchError::Cancelled)) {
        drain_buffered_events(
            &mut events,
            output_callback,
            summary_callback,
            &mut summary_ordinal,
            provider_lifecycle,
        );
    }
    run_result
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
/// - `provider_lifecycle`: Optional secret-free event sink bound to the
///   invocation's catalog revision.
pub(super) fn forward_event(
    event: AgentEvent,
    output_callback: Option<&crate::OutputCallback>,
    summary_callback: Option<&crate::ReasoningSummaryCallback>,
    summary_ordinal: &mut u64,
    provider_lifecycle: Option<&ProviderLifecycleContext>,
) {
    match event {
        AgentEvent::ProviderAuthentication { result } => {
            if let Some(lifecycle) = provider_lifecycle {
                lifecycle
                    .events
                    .report_agent_result(result, lifecycle.catalog_revision);
            }
        }
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
/// - `provider_lifecycle`: Optional secret-free event sink bound to the
///   invocation's catalog revision.
pub(super) fn drain_buffered_events(
    events: &mut tokio::sync::broadcast::Receiver<AgentEvent>,
    output_callback: Option<&crate::OutputCallback>,
    summary_callback: Option<&crate::ReasoningSummaryCallback>,
    summary_ordinal: &mut u64,
    provider_lifecycle: Option<&ProviderLifecycleContext>,
) {
    loop {
        match events.try_recv() {
            Ok(event) => forward_event(
                event,
                output_callback,
                summary_callback,
                summary_ordinal,
                provider_lifecycle,
            ),
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
