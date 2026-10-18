//! Resumes remote event observation and refreshes known sessions after gaps.

use super::{RemoteAdapter, service_remote_mapping::map_remote_event};
use scriberig_app::ApplicationError;

/// Resumes the event stream and reconstructs known session state after expiry.
///
/// # Parameters
/// - `adapter`: Remote service holding the cursor and known session identities.
///
/// # Returns
/// Decoded remote events plus authoritative session refresh notifications.
///
/// # Errors
/// Returns a sanitized remote protocol error.
pub(super) fn application_events<T: scriberig_protocol::ProtocolTransport>(
    adapter: &RemoteAdapter<T>,
) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
    let cursor = adapter.event_cursor.lock().ok().and_then(|value| *value);
    let mut refresh_sessions = Vec::new();
    let mut cursor_expired = false;
    let events = match adapter.client.subscribe_after(Vec::new(), cursor) {
        Ok(events) => events,
        Err(scriberig_client::ClientError::Protocol(error))
            if super::service_remote_mapping::protocol_category(&error.code)
                == "cursor_too_old" =>
        {
            cursor_expired = true;
            let events = adapter
                .client
                .subscribe_after(Vec::new(), None)
                .map_err(super::service_remote_mapping::remote_error)?;
            refresh_sessions = adapter
                .known_sessions
                .lock()
                .map(|sessions| sessions.iter().copied().collect())
                .unwrap_or_default();
            events
        }
        Err(error) => return Err(super::service_remote_mapping::remote_error(error)),
    };
    if let Some(sequence) = events.iter().map(|event| event.sequence).max() {
        if let Ok(mut cursor) = adapter.event_cursor.lock() {
            *cursor = Some(sequence);
        }
    } else if cursor_expired && let Ok(mut cursor) = adapter.event_cursor.lock() {
        *cursor = None;
    }
    let mut application_events = events
        .into_iter()
        .filter_map(|event| map_remote_event(&event))
        .collect::<Vec<_>>();
    application_events.extend(
        refresh_sessions
            .into_iter()
            .map(|session| scriberig_app::ApplicationEvent::SessionChanged { session }),
    );
    Ok(application_events)
}
