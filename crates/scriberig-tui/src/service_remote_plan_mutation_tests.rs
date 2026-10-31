//! Verifies TUI plan mutations use the shared remote protocol operation.

use std::sync::{Arc, Mutex};

use crate::service::{RemoteAdapter, TuiService};
use scriberig_app::{ApplicationContext, PlanMutation, PlanMutationRequest};
use scriberig_client::Client;
use scriberig_core::EntityId;
use scriberig_protocol::{
    ProtocolError, ProtocolErrorCode, ProtocolTransport, ProtocolVersion, Request, RequestEnvelope,
    Response, ResponseEnvelope,
};

/// Captures the protocol request while returning a deterministic permission error.
struct CapturingTransport {
    /// Last request received from the TUI client.
    request: Arc<Mutex<Option<Request>>>,
}

impl ProtocolTransport for CapturingTransport {
    /// Captures one request and refuses it before server-side mutation.
    ///
    /// - `self`: Shared request capture.
    /// - `request`: Typed protocol request produced by the remote adapter.
    ///
    /// A permission error response used to finish the transport call.
    /// # Returns
    /// The correlated error envelope.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        *self.request.lock().expect("request capture") = Some(request.request);
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::Error(ProtocolError {
                code: ProtocolErrorCode::PermissionDenied,
                message: "fixture denied mutation".to_owned(),
                details: None,
                correlation_id: None,
            }),
        }
    }
}

/// Verifies a gate waiver preserves exact optimistic identity and reason remotely.
#[test]
fn remote_gate_waiver_uses_typed_shared_mutation_request() {
    let captured = Arc::new(Mutex::new(None));
    let transport = CapturingTransport {
        request: Arc::clone(&captured),
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let session = EntityId::from_bytes([181; 16]);
    let plan = EntityId::from_bytes([182; 16]);
    let gate = EntityId::from_bytes([183; 16]);
    let result = adapter.mutate_plan(
        &ApplicationContext::empty(),
        &PlanMutationRequest {
            session,
            plan,
            expected_revision: 7,
            mutation: PlanMutation::WaiveGate {
                gate,
                reason: "reviewed exception".to_owned(),
            },
        },
    );
    assert!(result.is_err(), "the fixture rejects the request");
    let request = captured
        .lock()
        .expect("request capture")
        .clone()
        .expect("protocol request was captured");
    let Request::PlanMutation(request) = request else {
        panic!("expected plan mutation request: {request:?}");
    };
    assert_eq!(request.session.id, session.to_string());
    assert_eq!(request.plan, plan.to_string());
    assert_eq!(request.expected_revision, 7);
    assert_eq!(
        request.mutation,
        scriberig_protocol::SessionPlanMutation::WaiveGate {
            gate: gate.to_string(),
            reason: "reviewed exception".to_owned(),
        }
    );
}
