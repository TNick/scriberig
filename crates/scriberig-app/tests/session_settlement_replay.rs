//! Verifies a replayed agent settlement cannot duplicate durable semantic activity.
//!
//! Upstream owns ordinary tool-call settlement and loop continuation, so the
//! same branch handle can reach completion or failure more than once when a
//! provider turn is replayed. `ScribeRig` owns the durable activity graph, so a
//! second settlement must be rejected before any mutation: model invocations,
//! tool invocation/result pairs, turns, activity entries, and the branch result
//! must stay exactly as the first settlement recorded them.

#[path = "session_runtime_support.rs"]
mod support;

use scriberig_app::{
    AgentCommandRequest, ApplicationError, ApplicationService, SessionBackend, SessionRequest,
    ToolExecutionEvidence,
};
use scriberig_core::EntityId;
use scriberig_session::{Branch, BranchStatus, ModelRequestProjection};
use support::{Fixture, session_context};

/// Rejects a repeated root completion and keeps the recorded graph identical.
#[test]
fn replayed_root_completion_does_not_duplicate_durable_activity() {
    let service = ApplicationService::new(Fixture);
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([61; 16]),
            branch: None,
            anchor: None,
            prompt: "run one semantic lookup".to_owned(),
            selection: None,
        })
        .expect("pending root operation");
    service
        .complete_local_agent_with_evidence(
            &root,
            "lookup completed",
            vec!["entity:demo".to_owned()],
            tool_evidence(),
            model_requests(),
        )
        .expect("first root settlement");
    let settled = durable_branch(&service, root.session, root.root_branch);

    let replay = service.complete_local_agent_with_evidence(
        &root,
        "lookup completed",
        vec!["entity:demo".to_owned()],
        tool_evidence(),
        model_requests(),
    );

    assert_rejection(replay.as_ref().err(), "local_agent_complete");
    assert_eq!(
        durable_branch(&service, root.session, root.root_branch),
        settled,
        "a replayed completion must not append activity, invocations, or results"
    );
}

/// Keeps a failure replay off a branch that already completed.
///
/// The upstream loop can report a terminal failure after a completion has been
/// recorded for the same handle; that late observation must not convert the
/// completed branch into an abandoned one or add a second branch result.
#[test]
fn late_root_failure_does_not_replace_a_completed_branch() {
    let service = ApplicationService::new(Fixture);
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([62; 16]),
            branch: None,
            anchor: None,
            prompt: "complete before the failure arrives".to_owned(),
            selection: None,
        })
        .expect("pending root operation");
    service
        .complete_local_agent(&root, "completed before the failure")
        .expect("first root settlement");
    let settled = durable_branch(&service, root.session, root.root_branch);

    let late_failure = service.fail_local_agent_with_evidence(
        &root,
        "late provider failure",
        model_requests(),
        tool_evidence(),
    );

    assert_rejection(late_failure.as_ref().err(), "local_agent_fail");
    let after = durable_branch(&service, root.session, root.root_branch);
    assert_eq!(after, settled);
    assert_eq!(after.status, BranchStatus::Completed);
    assert_eq!(
        after
            .result
            .as_ref()
            .expect("one durable branch result")
            .summary
            .as_deref(),
        Some("completed before the failure")
    );
}

/// Rejects a repeated child settlement so nested evidence stays single.
#[test]
fn replayed_child_settlement_does_not_duplicate_durable_activity() {
    let service = ApplicationService::new(Fixture);
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([63; 16]),
            branch: None,
            anchor: None,
            prompt: "delegate one child".to_owned(),
            selection: None,
        })
        .expect("root operation");
    let child = service
        .start_local_child_agent(
            root.session,
            root.root_branch,
            "architecture",
            "map the architecture",
            "architecture result",
        )
        .expect("child operation");
    service
        .complete_local_child_agent_with_handles(
            &child,
            "architecture result",
            vec!["branch:child".to_owned()],
        )
        .expect("first child settlement");
    let settled = durable_branch(&service, root.session, child.child_branch);

    let late_failure = service.fail_local_child_agent(&child, "late child failure");

    assert_rejection(late_failure.as_ref().err(), "local_child_agent_fail");
    let after = durable_branch(&service, root.session, child.child_branch);
    assert_eq!(after, settled);
    assert_eq!(after.status, BranchStatus::Completed);
}

/// Reloads one durable branch by identity.
///
/// - `service`: Application service holding the session.
/// - `session`: Canonical session identity containing the branch.
/// - `branch`: Exact branch identity to reload.
///
/// # Returns
/// The authoritative branch state.
///
/// # Panics
/// Panics when the session or branch is missing.
fn durable_branch(
    service: &ApplicationService<Fixture>,
    session: EntityId,
    branch: EntityId,
) -> Branch {
    let session = service
        .read_session(&session_context(), &SessionRequest { session })
        .expect("durable session");
    session
        .branches
        .iter()
        .find(|candidate| candidate.id == branch)
        .expect("branch exists")
        .clone()
}

/// Returns one settled model request plus one settled tool call.
///
/// # Returns
/// Bounded tool evidence for one settled call.
fn tool_evidence() -> Vec<ToolExecutionEvidence> {
    vec![ToolExecutionEvidence {
        ordinal: 1,
        name: "semantic_find".to_owned(),
        input: br#"{"query":"Greeter"}"#.to_vec(),
        presentation_input: Some(br#"{"query":"Greeter"}"#.to_vec()),
        presentation_label: Some("Find Greeter".to_owned()),
        category: Some(scriberig_session::ToolActivityKind::SemanticRead),
        output: br#"{"handles":["entity:demo"]}"#.to_vec(),
        failed: false,
        child_branch: None,
    }]
}

/// Returns one captured provider request.
///
/// # Returns
/// Bounded request projection for one provider turn.
fn model_requests() -> Vec<ModelRequestProjection> {
    vec![ModelRequestProjection {
        ordinal: 0,
        model: "local-model".to_owned(),
        provider_id: Some("provider-upstream".to_owned()),
        provider_profile: None,
        thinking_level: Some("off".to_owned()),
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
        content_complete: true,
        applied_settings: None,
        applied_settings_fingerprint: None,
        truncated: false,
    }]
}

/// Asserts a settlement replay is rejected as an invalid request.
///
/// - `error`: Rejection from the settlement call, if any.
/// - `operation`: Expected operation named by the rejection.
///
/// # Panics
/// Panics when there is no invalid-request rejection for `operation`.
fn assert_rejection(error: Option<&ApplicationError>, operation: &str) {
    match error {
        Some(ApplicationError::InvalidRequest {
            operation: reported,
            message,
        }) => {
            assert_eq!(reported, operation);
            assert!(
                message.contains("already settled"),
                "the rejection names the settled branch: {message}"
            );
        }
        other => panic!("expected invalid-request rejection, got {other:?}"),
    }
}
