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
    /// - `self`: In-process client fixture.
    /// - `request`: Correlated semantic request.
    ///
    /// The fixed response with matching correlation identity.
    /// # Returns
    /// The operation result.
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

/// Builds a minimal typed plan response for plan-client method tests.
/// # Returns
/// The operation result.
fn empty_plan() -> scriberig_protocol::SessionPlan {
    scriberig_protocol::SessionPlan {
        id: "plan-1".to_owned(),
        revision: 1,
        exact_revision: None,
        snapshot: None,
        goal: "goal".to_owned(),
        status: scriberig_protocol::SessionPlanStatus::Planned,
        anchor: None,
        root_branch: "branch-1".to_owned(),
        nodes: Vec::new(),
        dependencies: Vec::new(),
        root_nodes: Vec::new(),
        status_counts: scriberig_protocol::SessionPlanStatusCounts::default(),
        gates: Vec::new(),
        evidence: Vec::new(),
        summaries: Vec::new(),
        coordinator: None,
        coordinator_cursor: None,
        coordinator_claim: None,
        history: Vec::new(),
    }
}

/// Sends bounded plan attempt-history requests and returns the typed page.
#[test]
fn plan_attempt_history_method_preserves_snapshot_and_page_fields() {
    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::PlanAttemptHistory(scriberig_protocol::SessionPlanAttemptHistoryPage {
            offset: 4,
            attempts: Vec::new(),
        }),
        request: Arc::clone(&observed),
    });

    let page = client
        .plan_attempt_history(scriberig_protocol::SessionPlanAttemptHistoryRequest {
            session: scriberig_protocol::SessionReference {
                id: "session-1".to_owned(),
            },
            plan: "plan-1".to_owned(),
            node: "node-1".to_owned(),
            snapshot: "snapshot-1".to_owned(),
            offset: 4,
            limit: 12,
        })
        .expect("attempt page");

    assert_eq!(page.offset, 4);
    assert_eq!(
        page.attempts,
        [] as [scriberig_protocol::SessionPlanAttempt; 0]
    );
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::PlanAttemptHistory(request))
            if request.snapshot == "snapshot-1" && request.offset == 4 && request.limit == 12
    ));
}

/// Verifies semantic selection calls use revision-bearing protocol requests.
#[test]
fn selection_methods_preserve_mutation_revisions() {
    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::SessionExecution(SessionExecutionSelection {
            revision: 4,
            provider_id: Some("provider-a".to_owned()),
            model_id: Some("model-a".to_owned()),
            thinking_level: Some("careful".to_owned()),
            history: Vec::new(),
        }),
        request: Arc::clone(&observed),
    });

    let selection = client
        .set_session_model("session-a", 3, "provider-a", "model-a")
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
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
            }
        ))
    );
}

/// Sends semantic domain requests through the shared typed command envelope.
#[test]
fn domain_operation_uses_exact_protocol_command_payload() {
    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::Error(scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::UnsupportedOperation,
            message: "fixture".to_owned(),
            details: None,
            correlation_id: None,
        }),
        request: Arc::clone(&observed),
    });
    let _ = client.domain_operation(scriberig_protocol::DomainOperationCommand {
        capability: "code.add_function".to_owned(),
        entity: scriberig_protocol::EntityReference {
            id: "entity-exact".to_owned(),
        },
        snapshot: scriberig_protocol::SnapshotReference {
            id: "snapshot-exact".to_owned(),
        },
        revision: None,
        branch: None,
        arguments: [("name".to_owned(), serde_json::json!("greet"))]
            .into_iter()
            .collect(),
    });
    let request = observed.lock().expect("observed request");
    let Some(Request::Command(request)) = request.as_ref() else {
        panic!("domain request did not use command protocol");
    };
    assert_eq!(request.kind, "domain_operation");
    assert_eq!(request.payload["entity"]["id"], "entity-exact");
    assert_eq!(request.payload["snapshot"]["id"], "snapshot-exact");
}

/// Sends application allocated new-session requests without local identities.
#[test]
fn new_session_request_uses_explicit_protocol_variant() {
    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::Error(scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::UnsupportedOperation,
            message: "fixture".to_owned(),
            details: None,
            correlation_id: None,
        }),
        request: Arc::clone(&observed),
    });
    let _ = client.start_new_session_agent("start", None);
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::AgentNewSession(request)) if request.prompt == "start"
    ));
}

/// Sends exact anchor identity in a dedicated remote continuation request.
#[test]
fn exact_continuation_request_keeps_historical_anchor() {
    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::Error(scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::UnsupportedOperation,
            message: "fixture".to_owned(),
            details: None,
            correlation_id: None,
        }),
        request: Arc::clone(&observed),
    });
    let anchor = scriberig_protocol::SessionContinuationAnchor::HumanInput {
        branch: "branch-a".to_owned(),
        turn: "turn-a".to_owned(),
    };
    let _ = client.continue_agent_at("session-a", anchor.clone(), "follow up", None);
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::AgentContinueAt(request))
            if request.anchor == anchor && request.prompt == "follow up"
    ));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The test exercises each typed plan client request through one fixture."
)]
/// Uses typed plan creation, lifecycle, assignment, and retry operations.
fn plan_client_methods_send_shared_application_requests() {
    let observed = Arc::new(Mutex::new(None));
    let client = Client::new(FixtureTransport {
        response: Response::Plan(empty_plan()),
        request: Arc::clone(&observed),
    });
    let create = scriberig_protocol::SessionPlanCreateRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        anchor: scriberig_protocol::SessionContinuationAnchor::BranchHead {
            branch: "branch-1".to_owned(),
        },
        goal: "goal".to_owned(),
        nodes: Vec::new(),
        dependencies: Vec::new(),
    };
    assert!(client.create_plan(create).is_ok());
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::PlanCreate(_))
    ));
    assert!(
        client
            .plan_lifecycle(scriberig_protocol::SessionPlanLifecycleRequest {
                session: SessionReference {
                    id: "session-1".to_owned(),
                },
                plan: "plan-1".to_owned(),
                expected_revision: 1,
                action: scriberig_protocol::SessionPlanAction::Start,
            })
            .is_ok()
    );
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::PlanLifecycle(_))
    ));
    assert!(
        client
            .set_plan_assignment(scriberig_protocol::SessionPlanAssignmentRequest {
                session: SessionReference {
                    id: "session-1".to_owned(),
                },
                plan: "plan-1".to_owned(),
                node: "node-1".to_owned(),
                expected_revision: 1,
                assignment: None,
            })
            .is_ok()
    );
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::PlanAssignment(_))
    ));
    assert!(
        client
            .retry_plan_node(scriberig_protocol::SessionPlanRetryRequest {
                session: SessionReference {
                    id: "session-1".to_owned(),
                },
                plan: "plan-1".to_owned(),
                node: "node-1".to_owned(),
                expected_revision: 2,
            })
            .is_ok()
    );
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::PlanRetry(_))
    ));
    assert!(
        client
            .add_plan_next_step(scriberig_protocol::SessionPlanNextStepRequest {
                session: SessionReference {
                    id: "session-1".to_owned(),
                },
                plan: "plan-1".to_owned(),
                expected_revision: 3,
                title: "next".to_owned(),
                purpose: "do follow-up work".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            })
            .is_ok()
    );
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::PlanNextStep(_))
    ));
    assert!(
        client
            .decompose_plan_node(scriberig_protocol::SessionPlanDecomposeRequest {
                session: SessionReference {
                    id: "session-1".to_owned(),
                },
                plan: "plan-1".to_owned(),
                parent: "node-1".to_owned(),
                expected_revision: 4,
                children: vec![scriberig_protocol::SessionPlanChildDraft {
                    title: "child".to_owned(),
                    purpose: "work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                required: true,
            })
            .is_ok()
    );
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::PlanDecompose(_))
    ));
    assert!(
        client
            .mutate_plan_semantics(scriberig_protocol::SessionPlanSemanticMutationRequest {
                session: SessionReference {
                    id: "session-1".to_owned(),
                },
                plan: "plan-1".to_owned(),
                expected_revision: 5,
                mutation: scriberig_protocol::SessionPlanSemanticMutation::UpdateSummary {
                    node: "node-1".to_owned(),
                    text: "Progress summary".to_owned(),
                    sources: Vec::new(),
                },
            },)
            .is_ok()
    );
    assert!(matches!(
        observed.lock().expect("request").as_ref(),
        Some(Request::PlanSemanticMutation(_))
    ));
}
