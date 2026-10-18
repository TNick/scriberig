//! Verifies revision-aware agent selection calls in the protocol client.

use std::sync::{Arc, Mutex};

use super::*;
use scriberig_protocol::{ProtocolVersion, RequestEnvelope, ResponseEnvelope};

/// Returns a fixed response and records the semantic request.
#[derive(Clone)]
struct FixtureTransport {
    /// Response returned to the client call.
    response: Response,
    /// Last typed request received by the fixture.
    request: Arc<Mutex<Option<Request>>>,
}

impl ProtocolTransport for FixtureTransport {
    /// Records the request and returns the configured response.
    ///
    /// # Parameters
    /// - `self`: In-process client fixture.
    /// - `request`: Correlated semantic request.
    ///
    /// # Returns
    /// The fixed response with matching correlation identity.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        if let Ok(mut observed) = self.request.lock() {
            *observed = Some(request.request);
        }
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: self.response.clone(),
        }
    }
}

/// Verifies semantic selection calls use revision-bearing protocol requests.
#[test]
fn selection_methods_preserve_mutation_revisions() {
    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::SessionExecution(SessionExecutionSelection {
            revision: 4,
            model_profile: Some("profile-a".to_owned()),
            model: Some("model-a".to_owned()),
            thinking_level: Some("careful".to_owned()),
            history: Vec::new(),
        }),
        request: Arc::clone(&observed),
    });

    let selection = client
        .set_session_model("session-a", 3, "profile-a")
        .expect("selection response");

    assert_eq!(selection.revision, 4);
    assert_eq!(
        observed.lock().expect("observed request").as_ref(),
        Some(&Request::SessionModelSelection(
            SessionModelSelectionRequest {
                session: SessionReference {
                    id: "session-a".to_owned(),
                },
                expected_revision: 3,
                model_profile: "profile-a".to_owned(),
            }
        ))
    );
}
