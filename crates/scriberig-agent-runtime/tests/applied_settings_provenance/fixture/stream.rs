//! Builds a completed response stream for applied-settings fixtures.

use std::sync::Arc;

use rpi_ai::Model;
use rpi_ai::providers::faux::faux_text;
use rpi_ai::types::{AssistantMessage, AssistantMessageEvent, DoneReason, StopReason};

/// Streams one completed text response for the bound model.
///
/// # Parameters
/// - `model`: Exact model that produced the response.
/// - `provider_id`: Provider identity carried by the response.
///
/// # Returns
/// The completed response event stream.
pub(super) fn completed_stream(
    model: &Model,
    provider_id: &str,
) -> rpi_ai::AssistantMessageEventStream {
    let (mut producer, stream) = rpi_ai::event_stream::create_assistant_message_event_stream();
    let message = AssistantMessage {
        content: vec![faux_text("safe completed answer")],
        stop_reason: StopReason::Stop,
        ..AssistantMessage::empty(
            model.api.clone(),
            provider_id.to_owned(),
            model.id.clone(),
            0,
        )
    };
    producer.push(AssistantMessageEvent::Start {
        partial: Arc::new(message.clone()),
    });
    producer.push(AssistantMessageEvent::TextStart {
        content_index: 0,
        partial: Arc::new(message.clone()),
    });
    producer.push(AssistantMessageEvent::TextEnd {
        content_index: 0,
        content: "safe completed answer".to_owned(),
        partial: Arc::new(message.clone()),
    });
    producer.push(AssistantMessageEvent::Done {
        reason: DoneReason::Stop,
        message,
    });
    stream
}
