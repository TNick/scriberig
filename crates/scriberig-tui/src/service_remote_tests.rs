//! Verifies remote TUI protocol mapping and adapter behavior.

use super::super::service_remote_mapping::{
    map_remote_operation, remote_error, session_from_response,
};
use super::RemoteAdapter;
use crate::service::TuiService;
use scriberig_app::ApplicationContext;
use scriberig_client::Client;
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_protocol::{
    AgentAcceptedResponse, CapabilitiesResponse, CapabilityInfo, EntityReference,
    InspectEntityResponse, Operation, OperationStatusResponse, ProtocolTransport, ProtocolVersion,
    RequestEnvelope, Response, ResponseEnvelope, RevisionReference, RuntimeFeature,
    SessionActivityItem, SessionActivityKind, SessionActivityState, SessionBranch,
    SessionBranchStatus, SessionReadResponse, SessionReference, SessionTurn, SnapshotReference,
};

#[path = "service_remote_plan_mutation_tests.rs"]
mod plan_mutation_tests;
#[path = "service_remote_reconnect_tests.rs"]
mod reconnect_tests;
#[path = "service_remote_session_event_tests.rs"]
mod session_event_tests;

/// Returns one deterministic inspect response without network access.
#[derive(Clone)]
struct FixtureTransport {
    response: ResponseEnvelope,
}

impl ProtocolTransport for FixtureTransport {
    /// Returns the fixture response for every protocol request.
    ///
    /// - `self`: Fixture transport.
    /// - `_request`: Protocol request received from the client.
    ///
    /// The deterministic response envelope.
    /// # Returns
    /// The operation result.
    fn send(&self, _request: RequestEnvelope) -> ResponseEnvelope {
        self.response.clone()
    }
}

/// Verifies remote inspection projects the same stable identities as local inspection.
#[test]
fn remote_inspection_preserves_semantic_handles() {
    let entity = EntityId::from_bytes([1; 16]);
    let revision = RevisionId::new([2; 32]);
    let snapshot = SnapshotId::new([3; 32]);
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "test".to_owned(),
            response: Response::Inspect(InspectEntityResponse {
                entity: EntityReference {
                    id: entity.to_string(),
                },
                revision: RevisionReference {
                    entity: EntityReference {
                        id: entity.to_string(),
                    },
                    id: revision.to_string(),
                },
                snapshot: SnapshotReference {
                    id: snapshot.to_string(),
                },
                object_type: Some("markdown.document".to_owned()),
                summary: Some("fixture".to_owned()),
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let result = adapter
        .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
        .expect("remote inspection");
    assert_eq!(result.entity, entity);
    assert_eq!(result.revision, Some(revision));
    assert_eq!(result.snapshot, Some(snapshot));
}

/// Preserves plans and explicit dependency results from a remote session read.
/// The fixture also exercises gate, evidence, summary, and root-order mapping.
#[test]
#[allow(clippy::too_many_lines)]
fn remote_session_mapping_preserves_plan_results_and_assignments() {
    let root = EntityId::from_bytes([51; 16]);
    let prerequisite = EntityId::from_bytes([52; 16]);
    let dependent = EntityId::from_bytes([53; 16]);
    let result = EntityId::from_bytes([54; 16]);
    let response = SessionReadResponse {
        session: SessionReference {
            id: EntityId::from_bytes([50; 16]).to_string(),
        },
        branches: Vec::new(),
        plans: vec![scriberig_protocol::SessionPlan {
            id: EntityId::from_bytes([55; 16]).to_string(),
            revision: 2,
            goal: "test plan".to_owned(),
            status: scriberig_protocol::SessionPlanStatus::Running,
            anchor: Some(scriberig_protocol::SessionContinuationAnchor::BranchHead {
                branch: root.to_string(),
            }),
            root_branch: root.to_string(),
            nodes: vec![scriberig_protocol::SessionPlanNode {
                id: prerequisite.to_string(),
                parent: root.to_string(),
                title: "producer".to_owned(),
                purpose: "produce a semantic result".to_owned(),
                status: scriberig_protocol::SessionPlanNodeStatus::Completed,
                exact_revision: None,
                effective_status: None,
                contains_ignored: false,
                contains_abandoned: false,
                abandon_reason: None,
                branch: None,
                result: Some(result.to_string()),
                superseded_by: None,
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: Some(scriberig_protocol::SessionModelAssignment {
                    provider_id: "provider-a".to_owned(),
                    model_id: "model-a".to_owned(),
                    thinking_level: Some("careful".to_owned()),
                }),
                claim: None,
                summary_id: None,
                summary_preview: None,
                dependency_blockers: Vec::new(),
                entry_gates: scriberig_protocol::SessionPlanGateCounts::default(),
                completion_gates: scriberig_protocol::SessionPlanGateCounts::default(),
                evidence_count: 0,
                can_retry: false,
                last_failure: None,
            }],
            dependencies: vec![scriberig_protocol::SessionPlanDependency {
                prerequisite: prerequisite.to_string(),
                dependent: dependent.to_string(),
                fulfilled_by: Some(result.to_string()),
            }],
            coordinator: Some(root.to_string()),
            coordinator_cursor: Some(dependent.to_string()),
            coordinator_claim: None,
            history: Vec::new(),
            exact_revision: None,
            snapshot: None,
            root_nodes: vec![dependent.to_string()],
            status_counts: scriberig_protocol::SessionPlanStatusCounts::default(),
            gates: vec![scriberig_protocol::SessionPlanGate {
                id: EntityId::from_bytes([56; 16]).to_string(),
                owner: dependent.to_string(),
                phase: scriberig_protocol::SessionPlanGatePhase::Completion,
                requirement: scriberig_protocol::SessionPlanGateRequirement::EvidencePresent {
                    kind: scriberig_protocol::SessionPlanEvidenceKind::Artifact,
                },
                waiver: None,
            }],
            evidence: vec![scriberig_protocol::SessionPlanEvidenceAssociation {
                id: EntityId::from_bytes([57; 16]).to_string(),
                owner: scriberig_protocol::SessionPlanEvidenceOwner::Gate(
                    EntityId::from_bytes([56; 16]).to_string(),
                ),
                role: scriberig_protocol::SessionPlanEvidenceRole::Supports,
                evidence: scriberig_protocol::SessionPlanEvidenceReference {
                    kind: scriberig_protocol::SessionPlanEvidenceKind::Artifact,
                    reference: scriberig_protocol::SessionSemanticReference {
                        entity: EntityId::from_bytes([58; 16]).to_string(),
                        revision: None,
                        snapshot: None,
                    },
                    attributes: std::collections::BTreeMap::new(),
                },
                note: None,
                actor: None,
            }],
            summaries: vec![scriberig_protocol::SessionPlanSummary {
                id: EntityId::from_bytes([59; 16]).to_string(),
                node: dependent.to_string(),
                text: "remote summary".to_owned(),
                sources: Vec::new(),
            }],
        }],
        execution: scriberig_protocol::SessionExecutionSelection::default(),
    };

    let session = session_from_response(response).expect("mapped session");
    assert_eq!(session.plans[0].dependencies[0].fulfilled_by, Some(result));
    assert_eq!(session.plans[0].coordinator, Some(root));
    assert_eq!(session.plans[0].revision, 2);
    assert_eq!(session.plans[0].goal, "test plan");
    assert_eq!(session.plans[0].root_nodes, vec![dependent]);
    assert_eq!(session.plans[0].gates.len(), 1);
    assert_eq!(session.plans[0].evidence.len(), 1);
    assert_eq!(session.plans[0].summaries[0].text, "remote summary");
    assert_eq!(
        session.plans[0].status,
        scriberig_session::PlanStatus::Running
    );
    assert_eq!(
        session.plans[0].nodes[0]
            .assignment
            .as_ref()
            .unwrap()
            .thinking_level
            .as_deref(),
        Some("careful")
    );
}

/// Verifies transport and semantic remote failures retain distinct categories.
#[test]
fn remote_errors_remain_categorized() {
    let local_denial = scriberig_app::ApplicationError::PermissionDenied {
        action: "session.continue".to_owned(),
        scope: "session-1".to_owned(),
    };
    let remote_denial = remote_error(scriberig_client::ClientError::Protocol(Box::new(
        scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::PermissionDenied,
            message: "permission denied".to_owned(),
            details: None,
            correlation_id: None,
        },
    )));
    assert_eq!(local_denial.category(), "permission_denied");
    assert_eq!(remote_denial.category(), local_denial.category());

    let transport = remote_error(scriberig_client::ClientError::Transport(
        "connection refused".to_owned(),
    ));
    assert_eq!(transport.category(), "transport_unavailable");
    let semantic = remote_error(scriberig_client::ClientError::Protocol(Box::new(
        scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::NotFound,
            message: "missing".to_owned(),
            details: None,
            correlation_id: None,
        },
    )));
    assert_eq!(semantic.category(), "not_found");
}

/// Verifies remote agent submission returns the shared operation identity.
#[test]
fn remote_agent_submission_uses_shared_operation() {
    let session = EntityId::from_bytes([4; 16]);
    let branch = EntityId::from_bytes([5; 16]);
    let operation = Operation {
        reference: scriberig_protocol::OperationReference {
            id: "op_agent_remote_1".to_owned(),
        },
        kind: scriberig_protocol::operation::OperationKind::Agent,
        status: scriberig_protocol::OperationStatus::Running,
        progress: None,
        result_snapshot: None,
        result_handles: Vec::new(),
        agent_result: None,
        error: None,
    };
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "agent".to_owned(),
            response: Response::AgentAccepted(AgentAcceptedResponse {
                operation: OperationStatusResponse { operation },
                session: SessionReference {
                    id: session.to_string(),
                },
                branch: Some(scriberig_protocol::BranchReference {
                    id: branch.to_string(),
                }),
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let result = adapter
        .submit_agent_command(&ApplicationContext::empty(), session, None, "inspect")
        .expect("agent operation");
    assert_eq!(result.to_string(), "op_agent_remote_1");
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "agent-handle".to_owned(),
            response: Response::AgentAccepted(AgentAcceptedResponse {
                operation: OperationStatusResponse {
                    operation: Operation {
                        reference: scriberig_protocol::OperationReference {
                            id: "op_agent_remote_2".to_owned(),
                        },
                        kind: scriberig_protocol::operation::OperationKind::Agent,
                        status: scriberig_protocol::OperationStatus::Running,
                        progress: None,
                        result_snapshot: None,
                        result_handles: Vec::new(),
                        agent_result: None,
                        error: None,
                    },
                },
                session: SessionReference {
                    id: session.to_string(),
                },
                branch: Some(scriberig_protocol::BranchReference {
                    id: branch.to_string(),
                }),
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let handle = adapter
        .submit_agent_handle(&ApplicationContext::empty(), session, None, "inspect")
        .expect("agent handles");
    assert_eq!(handle.operation.to_string(), "op_agent_remote_2");
    assert_eq!(handle.session, session);
    assert_eq!(handle.branch, Some(branch));
}

/// Uses remote new-session acceptance handles without local identity guesses.
#[test]
fn remote_new_session_handle_preserves_authoritative_identities() {
    let session = EntityId::from_bytes([31; 16]);
    let branch = EntityId::from_bytes([32; 16]);
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "new-session".to_owned(),
            response: Response::AgentAccepted(AgentAcceptedResponse {
                operation: OperationStatusResponse {
                    operation: Operation {
                        reference: scriberig_protocol::OperationReference {
                            id: "op_new_session_remote".to_owned(),
                        },
                        kind: scriberig_protocol::operation::OperationKind::Agent,
                        status: scriberig_protocol::OperationStatus::Running,
                        progress: None,
                        result_snapshot: None,
                        result_handles: Vec::new(),
                        agent_result: None,
                        error: None,
                    },
                },
                session: SessionReference {
                    id: session.to_string(),
                },
                branch: Some(scriberig_protocol::BranchReference {
                    id: branch.to_string(),
                }),
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));

    let handle = adapter
        .start_new_session_handle(&ApplicationContext::empty(), "first prompt", None)
        .expect("authoritative new-session handles");

    assert_eq!(handle.operation.to_string(), "op_new_session_remote");
    assert_eq!(handle.session, session);
    assert_eq!(handle.branch, Some(branch));
}

#[path = "service_remote_tests_part1.rs"]
mod part1;
