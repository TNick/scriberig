//! Verifies the provider-visible reasoning boundary of upstream events.

use super::event_forwarding::forward_event;
use rpi_agent::AgentEvent;
use rpi_ai::{Api, AssistantMessage, AssistantMessageEvent};
use std::sync::{Arc, Mutex};

/// Forwards public reasoning summaries while excluding raw thinking deltas.
#[test]
fn only_provider_visible_reasoning_summaries_reach_the_observer() {
    let partial = Arc::new(AssistantMessage::empty(Api::Faux, "provider", "model", 0));
    let output = Arc::new(Mutex::new(Vec::<String>::new()));
    let output_sink = Arc::clone(&output);
    let output_callback: crate::OutputCallback = Arc::new(move |delta| {
        output_sink
            .lock()
            .expect("output sink")
            .push(delta.to_owned());
    });
    let summaries = Arc::new(Mutex::new(Vec::<(u64, String)>::new()));
    let summary_sink = Arc::clone(&summaries);
    let summary_callback: crate::ReasoningSummaryCallback = Arc::new(move |ordinal, delta| {
        summary_sink
            .lock()
            .expect("summary sink")
            .push((ordinal, delta.to_owned()));
    });
    let mut ordinal = 0;

    for assistant_message_event in [
        AssistantMessageEvent::ReasoningSummaryDelta {
            content_index: 0,
            delta: "public summary".to_owned(),
            partial: Arc::clone(&partial),
        },
        AssistantMessageEvent::ThinkingDelta {
            content_index: 1,
            delta: "hidden chain of thought".to_owned(),
            partial: Arc::clone(&partial),
        },
        AssistantMessageEvent::TextDelta {
            content_index: 2,
            delta: "assistant answer".to_owned(),
            partial: Arc::clone(&partial),
        },
    ] {
        forward_event(
            AgentEvent::MessageUpdate {
                message: Arc::clone(&partial),
                assistant_message_event,
            },
            Some(&output_callback),
            Some(&summary_callback),
            &mut ordinal,
            None,
        );
    }

    assert_eq!(
        summaries.lock().expect("summary sink").as_slice(),
        &[(1, "public summary".to_owned())]
    );
    assert_eq!(
        output.lock().expect("output sink").as_slice(),
        &["assistant answer".to_owned()]
    );
}
