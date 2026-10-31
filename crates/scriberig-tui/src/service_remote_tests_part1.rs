//! Covers additional focused cases for the parent test module.

use super::*;
use crate::service::TuiRuntimeService;

/// Verifies remote operation preserves agent result handles.
#[test]
fn remote_operation_preserves_agent_result_handles() {
    let session = EntityId::from_bytes([8; 16]);
    let branch = EntityId::from_bytes([9; 16]);
    let branch_result = EntityId::from_bytes([10; 16]);
    let entity = EntityId::from_bytes([11; 16]);
    let snapshot = SnapshotId::new([12; 32]);
    let operation = Operation {
        reference: scriberig_protocol::OperationReference {
            id: "op_agent_result".to_owned(),
        },
        kind: scriberig_protocol::operation::OperationKind::Agent,
        status: scriberig_protocol::OperationStatus::Completed,
        progress: None,
        result_snapshot: None,
        result_handles: vec![branch_result.to_string()],
        agent_result: Some(scriberig_protocol::AgentResult {
            session: SessionReference {
                id: session.to_string(),
            },
            root_branch: scriberig_protocol::BranchReference {
                id: branch.to_string(),
            },
            final_output: "completed".to_owned(),
            branch_result: branch_result.to_string(),
            produced_changes: vec![EntityReference {
                id: entity.to_string(),
            }],
            validation_results: vec!["passed".to_owned()],
            unresolved_questions: Vec::new(),
            candidate_changes: vec![scriberig_protocol::CandidateChangeReference {
                migration: Some("migration-1".to_owned()),
                entity: Some(EntityReference {
                    id: entity.to_string(),
                }),
                candidate_snapshot: Some(SnapshotReference {
                    id: snapshot.to_string(),
                }),
                validation: vec!["validation-1".to_owned()],
            }],
        }),
        error: None,
    };

    let record = map_remote_operation(operation).expect("mapped operation");
    let agent_result = record.agent_result.expect("agent result");
    assert_eq!(agent_result.session, session);
    assert_eq!(agent_result.root_branch, branch);
    assert_eq!(agent_result.branch_result, branch_result);
    assert_eq!(agent_result.produced_changes, vec![entity]);
    assert_eq!(
        agent_result.candidate_changes[0].candidate_snapshot,
        Some(snapshot)
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "This regression validates the complete remote session/activity wire fixture and reconstructed branch ordering."
)]
/// Verifies remote session reads reconstruct ordered branch activity.
fn remote_session_read_reconstructs_branch_activity() {
    let session = EntityId::from_bytes([5; 16]);
    let branch = EntityId::from_bytes([6; 16]);
    let turn = EntityId::from_bytes([7; 16]);
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "session".to_owned(),
            response: Response::Session(SessionReadResponse {
                session: SessionReference {
                    id: session.to_string(),
                },
                branches: vec![SessionBranch {
                    id: branch.to_string(),
                    parent: None,
                    children: Vec::new(),
                    title: "main".to_owned(),
                    status: SessionBranchStatus::Active,
                    turns: vec![SessionTurn {
                        id: turn.to_string(),
                        ordinal: 3,
                        has_human_input: true,
                        has_agent_output: true,
                        agent_output: Some("fixture output".to_owned()),
                        tool_invocation: None,
                        tool_result: None,
                    }],
                    activity: vec![
                        SessionActivityItem {
                            id: format!("{turn}:human"),
                            session: session.to_string(),
                            sequence: 0,
                            branch: branch.to_string(),
                            operation: None,
                            parent: None,
                            kind: SessionActivityKind::HumanInput,
                            state: SessionActivityState::Completed,
                            summary: "Human input".to_owned(),
                            durable_ref: Some(turn.to_string()),
                            model: None,
                            thinking_level: None,
                            tool_name: None,
                            tool_category: None,
                            tool_arguments: None,
                            tool_result_preview: None,
                        },
                        SessionActivityItem {
                            id: format!("{turn}:assistant"),
                            session: session.to_string(),
                            sequence: 1,
                            branch: branch.to_string(),
                            operation: None,
                            parent: Some(format!("{turn}:human")),
                            kind: SessionActivityKind::AssistantOutput,
                            state: SessionActivityState::Completed,
                            summary: "fixture output".to_owned(),
                            durable_ref: Some(turn.to_string()),
                            model: None,
                            thinking_level: None,
                            tool_name: None,
                            tool_category: None,
                            tool_arguments: None,
                            tool_result_preview: None,
                        },
                    ],
                    result: None,
                }],
                plans: Vec::new(),
                execution: scriberig_protocol::SessionExecutionSelection {
                    revision: 4,
                    provider_id: Some("provider-a".to_owned()),
                    model_id: Some("model-a".to_owned()),
                    thinking_level: Some("careful".to_owned()),
                    history: Vec::new(),
                },
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let value = adapter
        .read_session(&ApplicationContext::empty(), session)
        .expect("remote session");
    assert_eq!(value.id, session);
    assert_eq!(value.branches.len(), 1);
    assert_eq!(value.branches[0].id, branch);
    assert_eq!(value.branches[0].turns[0].id, turn);
    assert_eq!(value.branches[0].turns[0].ordinal, 3);
    assert_eq!(value.execution.revision, 4);
    assert_eq!(value.execution.model_id.as_deref(), Some("model-a"));
    assert_eq!(value.execution.thinking_level.as_deref(), Some("careful"));
    assert!(value.branches[0].turns[0].human.is_some());
    assert!(value.branches[0].turns[0].agent.is_some());
    assert_eq!(
        value.branches[0].turns[0].agent.as_ref().unwrap().text,
        "fixture output"
    );
    assert_eq!(
        value.branches[0].activity,
        vec![
            scriberig_session::ExecutionActivity::HumanInput(turn),
            scriberig_session::ExecutionActivity::AgentOutput(turn),
        ]
    );
}

/// Verifies remote cancellation uses the shared operation lifecycle endpoint.
#[test]
fn remote_cancellation_returns_shared_lifecycle_state() {
    let operation = Operation {
        reference: scriberig_protocol::OperationReference {
            id: "op_cancel_remote_1".to_owned(),
        },
        kind: scriberig_protocol::operation::OperationKind::Agent,
        status: scriberig_protocol::OperationStatus::Cancelled,
        progress: None,
        result_snapshot: None,
        result_handles: Vec::new(),
        agent_result: None,
        error: None,
    };
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "cancel".to_owned(),
            response: Response::OperationCancelled(OperationStatusResponse { operation }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let record = adapter
        .cancel_operation(
            &ApplicationContext::empty(),
            &scriberig_core::OperationId::new("op_cancel_remote_1").expect("operation identity"),
        )
        .expect("remote cancellation");
    assert_eq!(record.status, scriberig_app::OperationStatus::Cancelled);
    assert_eq!(record.kind, scriberig_app::OperationKind::Agent);
}

/// Verifies remote capability discovery reaches the TUI action availability model.
#[test]
fn remote_capabilities_are_decoded_without_provider_details() {
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "capabilities".to_owned(),
            response: Response::Capabilities(CapabilitiesResponse {
                protocol: ProtocolVersion::default(),
                capabilities: vec![CapabilityInfo {
                    id: "agent_start".to_owned(),
                    description: "Start an agent operation".to_owned(),
                    available: true,
                    domain: Some("session".to_owned()),
                }],
                features: vec![RuntimeFeature {
                    id: "agent".to_owned(),
                    enabled: true,
                }],
                cad_providers: Vec::new(),
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let capabilities = adapter
        .runtime_capabilities(&ApplicationContext::empty())
        .expect("remote capabilities");
    assert_eq!(capabilities.len(), 1);
    assert_eq!(capabilities[0].id.to_string(), "agent_start");
    assert!(capabilities[0].available);
    assert_eq!(capabilities[0].scope.as_deref(), Some("session"));
    let palette = crate::overlay::palette::PaletteState::with_capabilities(&capabilities);
    assert!(
        palette
            .local_actions
            .iter()
            .any(|item| item.id == "start-agent")
    );
    assert!(
        !palette
            .local_actions
            .iter()
            .any(|item| item.id == "continue-agent")
    );
}

#[path = "service_adapter_conformance_tests.rs"]
mod adapter_conformance_tests;
