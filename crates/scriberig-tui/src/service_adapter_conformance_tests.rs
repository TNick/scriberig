//! Verifies local and remote TUI adapters expose equivalent semantics.

use std::sync::{Arc, Mutex};

use crate::service::TuiService;
use crate::service::{ApplicationAdapter, RemoteAdapter};
use scriberig_app::{
    AgentCommandRequest, ApplicationContext, ApplicationError, InspectRequest, InspectResult,
    QueryService, SessionBackend, SessionRequest,
};
use scriberig_client::Client;
use scriberig_core::EntityId;
use scriberig_protocol::{
    Event, EventKind, OperationReference, ProtocolTransport, ProtocolVersion, Request,
    RequestEnvelope, Response, ResponseEnvelope, SessionBranch, SessionBranchStatus,
    SessionReadResponse, SessionReference, SessionTurn,
};
use scriberig_session::{AgentOutput, Branch, HumanInput, Session, Turn};

#[path = "service_adapter_agent_console_tests.rs"]
mod agent_console_fixtures;

/// Supplies one deterministic local session to the application adapter.
#[derive(Clone)]
struct LocalSessionFixture {
    /// Authoritative session returned by the local boundary.
    session: Session,
}

impl QueryService for LocalSessionFixture {
    /// Returns the deterministic inspection represented by both adapters.
    ///
    /// - `_context`: Application request context.
    /// - `request`: Inspection request to resolve.
    ///
    /// The inspection preserves the requested semantic handles.
    ///
    /// This fixture does not return an application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Ok(InspectResult {
            entity: request.entity,
            revision: Some(scriberig_core::RevisionId::new([8; 32])),
            snapshot: match request.snapshot {
                scriberig_app::SnapshotResolution::At(snapshot) => Some(snapshot),
                _ => None,
            },
            object_type: Some(
                scriberig_core::ObjectType::new("fixture.entity").expect("object type"),
            ),
            label: Some("fixture inspection".to_owned()),
            description: None,
            capabilities: Vec::new(),
            trust: None,
        })
    }
}

impl SessionBackend for LocalSessionFixture {
    /// Returns the fixture's application allocated new-session handles.
    ///
    /// - `self`: Local session fixture.
    /// - `_context`: Application request context.
    /// - `_request`: First user instruction.
    ///
    /// Stable operation, session, and root-branch handles.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn start_new_session_agent(
        &self,
        _context: &ApplicationContext,
        _request: scriberig_app::NewSessionAgentRequest,
    ) -> Result<scriberig_app::NewSessionAgentStart, ApplicationError> {
        Ok(scriberig_app::NewSessionAgentStart {
            operation: scriberig_core::OperationId::new("local_new_session").expect("operation"),
            session: self.session.id,
            root_branch: self.session.branches[0].id,
        })
    }

    /// Returns a fixture continuation branch for exact-anchor adapter checks.
    ///
    /// - `self`: Local session fixture.
    /// - `_context`: Application request context.
    /// - `request`: Exact continuation request.
    ///
    /// Stable operation, session, and branch handles.
    ///
    /// Returns invalid-request for an anchor outside the fixture session.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn continue_agent_at(
        &self,
        _context: &ApplicationContext,
        request: scriberig_app::AgentContinueAtRequest,
    ) -> Result<scriberig_app::AgentContinuationAccepted, ApplicationError> {
        if self.session.resolve_anchor(request.anchor).is_err() {
            return Err(ApplicationError::InvalidRequest {
                operation: "continue_at".to_owned(),
                message: "unknown fixture anchor".to_owned(),
            });
        }
        Ok(scriberig_app::AgentContinuationAccepted {
            operation: scriberig_core::OperationId::new("local_continue_at").expect("operation"),
            session: request.session,
            branch: EntityId::from_bytes([33; 16]),
        })
    }

    /// Returns the deterministic session snapshot.
    ///
    /// - `_context`: Application request context.
    /// - `request`: Session identity requested by the adapter.
    ///
    /// The fixture session when the identity matches.
    ///
    /// Returns not-found for another session identity.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        request: &SessionRequest,
    ) -> Result<Session, ApplicationError> {
        if request.session == self.session.id {
            Ok(self.session.clone())
        } else {
            Err(ApplicationError::NotFound {
                subject: request.session.to_string(),
            })
        }
    }

    /// Returns one deterministic operation for adapter handle tests.
    ///
    /// - `_context`: Application request context.
    /// - `_request`: Accepted agent request.
    ///
    /// A stable fixture operation identity.
    ///
    /// This fixture never rejects submission.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _request: &AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        Ok(scriberig_core::OperationId::new("op_local_fixture").expect("operation"))
    }
}

impl scriberig_app::ApplicationBackend for LocalSessionFixture {
    /// Rejects unrelated operations in the local operation conformance fixture.
    ///
    /// - `self`: Deterministic local fixture.
    /// - `_context`: Authorized application context.
    /// - `_request`: Operation outside the lifecycle-only test's scope.
    ///
    /// An unsupported-operation error for all semantic executions.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::ApplicationRequest,
    ) -> Result<scriberig_app::ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "conformance fixture execution".to_owned(),
        })
    }
}

/// Supplies the fixed protocol response used by this adapter test.
#[derive(Clone)]
struct FixtureTransport {
    /// Response returned to every protocol request.
    response: ResponseEnvelope,
}

impl ProtocolTransport for FixtureTransport {
    /// Returns the deterministic response envelope.
    ///
    /// - `self`: Fixture transport.
    /// - `_request`: Protocol request received by the fixture.
    ///
    /// The configured protocol response.
    /// # Returns
    /// The operation result.
    fn send(&self, _request: RequestEnvelope) -> ResponseEnvelope {
        self.response.clone()
    }
}

/// Builds the semantic session represented by the protocol fixture.
///
/// A deterministic session with one active branch and one completed turn.
/// # Returns
/// The operation result.
fn session_fixture() -> Session {
    let session = EntityId::from_bytes([5; 16]);
    let branch = EntityId::from_bytes([6; 16]);
    let turn = EntityId::from_bytes([7; 16]);
    let mut root = Branch::new(branch, "main").expect("branch");
    root.push_turn(Turn {
        id: turn,
        ordinal: 3,
        human: Some(HumanInput {
            text: "inspect".to_owned(),
        }),
        agent: Some(AgentOutput {
            text: "fixture output".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    let plan_id = EntityId::from_bytes([8; 16]);
    let producer_id = EntityId::from_bytes([9; 16]);
    let consumer_id = EntityId::from_bytes([10; 16]);
    let result_id = EntityId::from_bytes([11; 16]);
    Session {
        id: session,
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![scriberig_session::Plan {
            id: plan_id,
            session: Some(session),
            anchor: Some(scriberig_session::ContinuationAnchor::BranchHead { branch }),
            goal: "inspect fixture semantics".to_owned(),
            revision: 2,
            status: scriberig_session::PlanStatus::Planned,
            root_branch: branch,
            nodes: vec![
                scriberig_session::PlanNode {
                    id: producer_id,
                    parent: Some(scriberig_session::PlanNodeParent::Branch(branch)),
                    title: "produce".to_owned(),
                    purpose: "produce result".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    status: scriberig_session::PlanNodeStatus::Completed,
                    branch: None,
                    result: Some(result_id),
                    superseded_by: None,
                    assignment: None,
                    claim: None,
                    claim_generation: 0,
                },
                scriberig_session::PlanNode {
                    id: consumer_id,
                    parent: Some(scriberig_session::PlanNodeParent::Branch(branch)),
                    title: "consume".to_owned(),
                    purpose: "consume result".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    status: scriberig_session::PlanNodeStatus::Ready,
                    branch: None,
                    result: None,
                    superseded_by: None,
                    assignment: None,
                    claim: None,
                    claim_generation: 0,
                },
            ],
            dependencies: vec![scriberig_session::PlanDependency {
                prerequisite: producer_id,
                dependent: consumer_id,
                fulfilled_by: Some(result_id),
            }],
            coordinator: None,
            coordinator_cursor: None,
            coordinator_claim: None,
            coordinator_generation: 0,
            root_nodes: Vec::new(),
            history: Vec::new(),
        }],
        execution: scriberig_session::SessionExecutionSelection {
            revision: 1,
            provider_id: Some("provider-fixture".to_owned()),
            model_id: Some("model-fixture".to_owned()),
            thinking_level: Some("careful".to_owned()),
            history: Vec::new(),
        },
    }
}

/// Verifies local and remote inspection preserve the same semantic result.
#[test]
fn local_and_remote_inspection_share_semantics() {
    let entity = EntityId::from_bytes([30; 16]);
    let snapshot = scriberig_core::SnapshotId::new([31; 32]);
    let local = ApplicationAdapter::new(
        LocalSessionFixture {
            session: session_fixture(),
        },
        Vec::new(),
    )
    .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
    .expect("local inspection");
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "inspect-equivalence".to_owned(),
            response: Response::Inspect(scriberig_protocol::InspectEntityResponse {
                entity: scriberig_protocol::EntityReference {
                    id: entity.to_string(),
                },
                revision: scriberig_protocol::RevisionReference {
                    entity: scriberig_protocol::EntityReference {
                        id: entity.to_string(),
                    },
                    id: scriberig_core::RevisionId::new([8; 32]).to_string(),
                },
                snapshot: scriberig_protocol::SnapshotReference {
                    id: snapshot.to_string(),
                },
                object_type: Some("fixture.entity".to_owned()),
                summary: Some("fixture inspection".to_owned()),
            }),
        },
    };
    let remote = RemoteAdapter::new(Client::new(transport))
        .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
        .expect("remote inspection");

    assert_eq!(local.entity, remote.entity);
    assert_eq!(local.revision, remote.revision);
    assert_eq!(local.snapshot, remote.snapshot);
    assert_eq!(local.object_type, remote.object_type);
    assert_eq!(local.label, remote.label);
}

/// Keeps TUI start and exact continuation adapters on shared handles.
#[test]
fn local_tui_adapter_uses_shared_start_and_exact_continuation() {
    let session = session_fixture();
    let session_id = session.id;
    let branch = session.branches[0].id;
    let anchor = scriberig_session::ContinuationAnchor::AssistantOutput {
        branch,
        turn: session.branches[0].turns[0].id,
    };
    let adapter = ApplicationAdapter::new(LocalSessionFixture { session }, Vec::new());

    let started = adapter
        .start_new_session_handle(&ApplicationContext::empty(), "first", None)
        .expect("shared start");
    let continued = adapter
        .continue_agent_at_handle(
            &ApplicationContext::empty(),
            session_id,
            anchor,
            "follow-up",
        )
        .expect("shared exact continuation");

    assert_eq!(started.session, session_id);
    assert_eq!(started.branch, Some(branch));
    assert_eq!(started.operation.to_string(), "op_local_new_session");
    assert_eq!(continued.session, session_id);
    assert_eq!(continued.operation.to_string(), "op_local_continue_at");
}

#[path = "service_adapter_conformance_tests_part1.rs"]
mod part1;
