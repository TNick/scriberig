//! Verifies remote cursor recovery refreshes durable session state.

use super::*;
use crate::service::TuiRuntimeService;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Returns a sequence of protocol responses for reconnect recovery tests.
struct QueuedTransport {
    /// Responses returned in request order.
    responses: Mutex<VecDeque<ResponseEnvelope>>,
    /// Requests received while recovering the event stream.
    requests: Arc<Mutex<Vec<RequestEnvelope>>>,
}

impl ProtocolTransport for QueuedTransport {
    /// Returns the next queued protocol response.
    ///
    /// - `self`: Queued test transport.
    /// - `request`: Protocol request received from the client.
    ///
    /// The next deterministic response envelope.
    /// # Returns
    /// The operation result.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        self.requests
            .lock()
            .expect("request queue lock")
            .push(request);
        self.responses
            .lock()
            .expect("response queue lock")
            .pop_front()
            .expect("queued protocol response")
    }
}

/// Verifies a lost event window causes replay and durable session refresh.
#[test]
fn expired_event_cursor_replays_and_refreshes_known_sessions() {
    let session = EntityId::from_bytes([14; 16]);
    let cursor_error = scriberig_protocol::ProtocolError {
        code: scriberig_protocol::ProtocolErrorCode::CursorTooOld,
        message: "event cursor expired".to_owned(),
        details: None,
        correlation_id: None,
    };
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = QueuedTransport {
        responses: Mutex::new(VecDeque::from([
            ResponseEnvelope {
                version: ProtocolVersion::default(),
                correlation_id: "old-cursor".to_owned(),
                response: Response::Error(cursor_error),
            },
            ResponseEnvelope {
                version: ProtocolVersion::default(),
                correlation_id: "replay".to_owned(),
                response: Response::Events(Vec::new()),
            },
        ])),
        requests: Arc::clone(&requests),
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    *adapter.event_cursor.lock().expect("event cursor") = Some(1);
    adapter
        .known_sessions
        .lock()
        .expect("known sessions")
        .insert(session);

    let events = adapter
        .application_events(&ApplicationContext::empty())
        .expect("recover after event gap");

    assert_eq!(
        events,
        vec![scriberig_app::ApplicationEvent::SessionChanged { session }]
    );
    assert_eq!(*adapter.event_cursor.lock().expect("event cursor"), None);
    let requests = requests.lock().expect("recorded requests");
    assert_eq!(requests.len(), 2);
    assert!(
        requests.iter().all(|request| {
            matches!(request.request, scriberig_protocol::Request::Subscribe(_))
        })
    );
}
