//! Verifies durable session activity for local and nested agent operations.

#[path = "session_runtime_support.rs"]
mod support;
use scriberig_app::{AgentCommandRequest, ApplicationService, SessionBackend, SessionRequest};
use scriberig_core::EntityId;
use support::{Fixture, child_model_request, session_context};
/// Verifies a local runtime can complete the shared operation with its output.
#[test]
fn local_agent_completion_uses_runtime_output() {
    let service = ApplicationService::new(Fixture);
    let session = EntityId::from_bytes([2; 16]);
    let handle = service
        .start_local_agent(&scriberig_app::AgentCommandRequest {
            session,
            branch: None,
            anchor: None,
            prompt: "instruction".to_owned(),
            selection: None,
        })
        .expect("pending agent operation");

    assert_eq!(
        service.operations().get(&handle.operation).unwrap().status,
        scriberig_app::OperationStatus::Running
    );
    let result = service
        .complete_local_agent(&handle, "runtime output")
        .expect("completed agent operation");

    assert_eq!(result.final_output, "runtime output");
    let state = service
        .read_session(
            &session_context(),
            &scriberig_app::SessionRequest { session },
        )
        .expect("session state");
    assert_eq!(
        state.branches[0].turns[0].human.as_ref().unwrap().text,
        "instruction"
    );
    assert_eq!(
        state.branches[0].turns[0].agent.as_ref().unwrap().text,
        "runtime output"
    );
    assert_eq!(
        state.branches[0].activity,
        vec![
            scriberig_session::ExecutionActivity::HumanInput(state.branches[0].turns[0].id),
            scriberig_session::ExecutionActivity::AgentOutput(state.branches[0].turns[0].id),
            scriberig_session::ExecutionActivity::BranchResult(
                state.branches[0].result.as_ref().unwrap().id,
            ),
        ]
    );
    assert_eq!(
        service.operations().get(&handle.operation).unwrap().status,
        scriberig_app::OperationStatus::Completed
    );
}

/// Verifies tool execution evidence is linked and retained by its durable branch.
#[test]
fn local_agent_completion_persists_tool_invocation_and_result() {
    let service = ApplicationService::new(Fixture);
    let handle = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([12; 16]),
            branch: None,
            anchor: None,
            prompt: "run a semantic lookup".to_owned(),
            selection: None,
        })
        .expect("pending agent operation");
    service
        .complete_local_agent_with_evidence(
            &handle,
            "lookup completed",
            vec!["entity:demo".to_owned()],
            vec![scriberig_app::ToolExecutionEvidence {
                ordinal: 1,
                name: "semantic_find".to_owned(),
                input: br#"{"query":"Greeter","token":"FAKE_SECRET_MARKER_8391"}"#.to_vec(),
                presentation_input: Some(br#"{"query":"Greeter","token":"[REDACTED]"}"#.to_vec()),
                presentation_label: Some("Find Greeter".to_owned()),
                category: Some(scriberig_session::ToolActivityKind::SemanticRead),
                output: br#"{"handles":["entity:demo"]}"#.to_vec(),
                failed: false,
                child_branch: None,
            }],
            vec![scriberig_session::ModelRequestProjection {
                ordinal: 0,
                model: "local-model".to_owned(),
                provider_id: Some("provider-upstream".to_owned()),
                provider_profile: None,
                thinking_level: Some("high".to_owned()),
                profile: None,
                active_components: Vec::new(),
                context_fingerprint: None,
                request_fingerprint: None,
                catalog_revision: None,
                upstream_runtime_revision: None,
                provider_response_id: None,
                model_execution_configuration: None,
                messages: vec![scriberig_session::ModelRequestMessage {
                    role: "user".to_owned(),
                    content: "safe request".to_owned(),
                }],
                full_messages: None,
                tools: Vec::new(),
                full_tools: None,
                content_complete: false,
                applied_settings: None,
                applied_settings_fingerprint: None,
                truncated: false,
            }],
        )
        .expect("complete agent with tool evidence");
    let session = service
        .read_session(
            &session_context(),
            &SessionRequest {
                session: handle.session,
            },
        )
        .expect("durable session");
    let model_invocation = &session.branches[0].model_invocations[0];
    assert_eq!(model_invocation.operation.as_ref(), Some(&handle.operation));
    assert_eq!(
        model_invocation.provider_id.as_deref(),
        Some("provider-upstream")
    );
    assert_eq!(model_invocation.provider_profile, None);
    assert_eq!(
        model_invocation.provider_model_id.as_deref(),
        Some("local-model")
    );
    assert_tool_evidence_is_persisted(&session);
}

/// Persists semantic mutation evidence with its candidate handle.
#[test]
fn semantic_operation_activity_retains_candidate_reference() {
    let service = ApplicationService::new(Fixture);
    let handle = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([17; 16]),
            branch: None,
            anchor: None,
            prompt: "rename the selected semantic entity".to_owned(),
            selection: None,
        })
        .expect("pending semantic operation");
    service
        .complete_local_agent_with_evidence(
            &handle,
            "semantic candidate created",
            vec!["candidate:rename-1".to_owned()],
            vec![scriberig_app::ToolExecutionEvidence {
                ordinal: 1,
                name: "domain_operation".to_owned(),
                input: br#"{"capability":"operation.rename"}"#.to_vec(),
                presentation_input: Some(br#"{"capability":"operation.rename"}"#.to_vec()),
                presentation_label: Some("Rename semantic entity".to_owned()),
                category: Some(scriberig_session::ToolActivityKind::SemanticMutation),
                output: br#"{"candidate":"candidate:rename-1"}"#.to_vec(),
                failed: false,
                child_branch: None,
            }],
            Vec::new(),
        )
        .expect("persist semantic operation evidence");
    let session = service
        .read_session(
            &session_context(),
            &SessionRequest {
                session: handle.session,
            },
        )
        .expect("durable semantic session");
    let branch = session
        .branches
        .iter()
        .find(|branch| branch.id == handle.root_branch)
        .expect("semantic operation branch");

    assert_eq!(branch.tool_invocations[0].name, "domain_operation");
    assert_eq!(
        branch.tool_invocations[0].category,
        Some(scriberig_session::ToolActivityKind::SemanticMutation)
    );
    assert_eq!(
        branch.tool_results[0].output,
        br#"{"candidate":"candidate:rename-1"}"#
    );
}

/// Checks persisted model, tool, and result evidence for one completed branch.
///
/// - `session`: Durable session returned after local agent completion.
///
/// # Panics
/// Panics when persisted model or tool evidence does not match the fixture.
fn assert_tool_evidence_is_persisted(session: &scriberig_session::Session) {
    let branch = &session.branches[0];
    assert_eq!(branch.model_invocations.len(), 1);
    assert_eq!(
        branch.model_invocations[0].model.as_deref(),
        Some("local-model")
    );
    assert_eq!(branch.model_invocations[0].ordinal, 0);
    assert_eq!(
        branch.model_invocations[0]
            .request
            .as_ref()
            .unwrap()
            .messages[0]
            .content,
        "safe request"
    );
    assert!(matches!(
        branch.activity[1],
        scriberig_session::ExecutionActivity::ModelInvocation(_)
    ));
    assert_eq!(branch.tool_invocations.len(), 1);
    assert_eq!(branch.tool_invocations[0].name, "semantic_find");
    assert_eq!(
        branch.tool_invocations[0].category,
        Some(scriberig_session::ToolActivityKind::SemanticRead)
    );
    assert_eq!(
        branch.tool_invocations[0].presentation_label.as_deref(),
        Some("Find Greeter")
    );
    assert_eq!(
        branch.tool_invocations[0].parent_model_invocation,
        Some(branch.model_invocations[0].id)
    );
    assert_eq!(
        branch.tool_invocations[0].input,
        br#"{"query":"Greeter","token":"FAKE_SECRET_MARKER_8391"}"#
    );
    assert_eq!(
        branch.tool_invocations[0].presentation_input.as_deref(),
        Some(br#"{"query":"Greeter","token":"[REDACTED]"}"#.as_slice())
    );
    assert_eq!(branch.tool_results.len(), 1);
    assert_eq!(
        branch.tool_results[0].invocation,
        branch.tool_invocations[0].id
    );
    assert_eq!(
        branch.tool_results[0].output,
        br#"{"handles":["entity:demo"]}"#
    );
    assert_eq!(
        branch.activity,
        vec![
            scriberig_session::ExecutionActivity::HumanInput(branch.turns[0].id),
            scriberig_session::ExecutionActivity::ModelInvocation(branch.model_invocations[0].id,),
            scriberig_session::ExecutionActivity::ToolInvocation(branch.tool_invocations[0].id),
            scriberig_session::ExecutionActivity::ToolResult(branch.tool_results[0].id),
            scriberig_session::ExecutionActivity::AgentOutput(branch.turns[0].id),
            scriberig_session::ExecutionActivity::BranchResult(branch.result.as_ref().unwrap().id,),
        ]
    );
}

/// Verifies child operation startup creates an explicit session graph edge.
#[test]
fn local_child_agent_start_creates_parent_link() {
    let service = ApplicationService::new(Fixture);
    let request = AgentCommandRequest {
        session: EntityId::from_bytes([21; 16]),
        branch: None,
        anchor: None,
        prompt: "root".to_owned(),
        selection: None,
    };
    let root = service.start_local_agent(&request).expect("root");
    let child = service
        .start_local_child_agent(
            root.session,
            root.root_branch,
            "child",
            "inspect architecture",
            "inspect",
        )
        .expect("child");
    let session = service
        .read_session(
            &session_context(),
            &SessionRequest {
                session: root.session,
            },
        )
        .expect("session");
    let parent = session
        .branches
        .iter()
        .find(|branch| branch.id == root.root_branch)
        .expect("parent");
    assert_eq!(parent.children, vec![child.child_branch]);
    assert!(parent.activity.iter().any(|activity| matches!(
        activity,
        scriberig_session::ExecutionActivity::ChildBranch(id) if *id == child.child_branch
    )));
    assert_eq!(
        session
            .branches
            .iter()
            .find(|branch| branch.id == child.child_branch)
            .and_then(|branch| branch.parent),
        Some(root.root_branch)
    );

    let completed = service
        .complete_local_child_agent_with_evidence(
            &child,
            "child output",
            vec!["project:prj-one".to_owned(), "snapshot:snp-base".to_owned()],
            Vec::new(),
            vec![child_model_request()],
        )
        .expect("complete child");
    assert_eq!(completed.root_branch, child.child_branch);
    let session = service
        .read_session(
            &session_context(),
            &SessionRequest {
                session: root.session,
            },
        )
        .expect("completed session");
    let child_branch = session
        .branches
        .iter()
        .find(|branch| branch.id == child.child_branch)
        .expect("child branch");
    assert_eq!(
        child_branch.status,
        scriberig_session::BranchStatus::Completed
    );
    assert_eq!(
        child_branch.model_invocations[0].operation.as_ref(),
        Some(&child.operation)
    );
    assert!(matches!(
        child_branch.activity.last(),
        Some(scriberig_session::ExecutionActivity::BranchResult(id))
            if Some(*id) == child_branch.result.as_ref().map(|result| result.id)
    ));
    assert_eq!(
        child_branch
            .result
            .as_ref()
            .expect("durable result")
            .result_handles,
        vec!["project:prj-one", "snapshot:snp-base"]
    );
    assert_eq!(
        child_branch
            .result
            .as_ref()
            .expect("durable result")
            .summary
            .as_deref(),
        Some("child output")
    );
}

#[path = "session_runtime_activity_parts/part1.rs"]
mod part1;
