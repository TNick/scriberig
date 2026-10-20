//! Verifies failed child results and captured requests remain durable.

use super::*;

/// Verifies failed child execution is retained as an abandoned branch result.
#[test]
fn failed_local_child_agent_is_retained() {
    let service = ApplicationService::new(Fixture);
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([31; 16]),
            branch: None,
            anchor: None,
            prompt: "root".to_owned(),
            selection: None,
        })
        .expect("root");
    let child = service
        .start_local_child_agent(
            root.session,
            root.root_branch,
            "child",
            "inspect",
            "child work",
        )
        .expect("child");

    service
        .fail_local_child_agent(&child, "provider failed")
        .expect("failure retained");

    let state = service
        .read_session(
            &session_context(),
            &SessionRequest {
                session: root.session,
            },
        )
        .expect("session");
    let failed = state
        .branches
        .iter()
        .find(|branch| branch.id == child.child_branch)
        .expect("failed child");
    assert_eq!(failed.status, scriberig_session::BranchStatus::Abandoned);
    assert_eq!(failed.outcome.as_deref(), Some("provider failed"));
    let result = failed.result.as_ref().expect("abandoned child result");
    assert_eq!(result.branch, child.child_branch);
    assert_eq!(result.status, scriberig_session::BranchStatus::Abandoned);
    assert_eq!(result.summary.as_deref(), Some("provider failed"));
    assert!(
        failed
            .activity
            .contains(&scriberig_session::ExecutionActivity::BranchResult(
                result.id
            ))
    );
    let parent = state
        .branches
        .iter()
        .find(|branch| branch.id == root.root_branch)
        .expect("root branch remains distinct");
    assert_eq!(parent.children, vec![child.child_branch]);
    assert!(parent.result.is_none());
    assert!(
        !parent
            .activity
            .contains(&scriberig_session::ExecutionActivity::BranchResult(
                result.id
            ))
    );
    assert_eq!(
        service.operations().get(&child.operation).unwrap().status,
        scriberig_app::OperationStatus::Failed
    );
}

/// Verifies failed child execution retains its bounded model request history.
#[test]
fn failed_local_child_agent_retains_safe_model_requests() {
    let service = ApplicationService::new(Fixture);
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([41; 16]),
            branch: None,
            anchor: None,
            prompt: "root".to_owned(),
            selection: None,
        })
        .expect("root");
    let child = service
        .start_local_child_agent(
            root.session,
            root.root_branch,
            "child",
            "inspect",
            "child work",
        )
        .expect("child");
    let request = scriberig_session::ModelRequestProjection {
        ordinal: 0,
        model: "model-public-id".to_owned(),
        provider_id: None,
        provider_profile: Some("provider-profile".to_owned()),
        thinking_level: Some("high".to_owned()),
        profile: Some(scriberig_session::ModelContextRevisionRef {
            identity: "profile.default".to_owned(),
            revision: "rev-3".to_owned(),
        }),
        active_components: vec![
            scriberig_session::ModelContextRevisionRef {
                identity: "scriberig.context-component.core-invariants".to_owned(),
                revision: "rev-5".to_owned(),
            },
            scriberig_session::ModelContextRevisionRef {
                identity: "scriberig.context-component.non-root-handoff".to_owned(),
                revision: "rev-8".to_owned(),
            },
        ],
        context_fingerprint: Some("context-fingerprint".to_owned()),
        request_fingerprint: Some("request-fingerprint".to_owned()),
        messages: vec![scriberig_session::ModelRequestMessage {
            role: "user".to_owned(),
            content: "child work".to_owned(),
        }],
        tools: Vec::new(),
        truncated: false,
    };

    service
        .fail_local_child_agent_with_model_requests(
            &child,
            "child agent execution failed",
            vec![request.clone()],
        )
        .expect("failure retained");

    let state = service
        .read_session(
            &session_context(),
            &SessionRequest {
                session: root.session,
            },
        )
        .expect("session");
    let failed = state
        .branches
        .iter()
        .find(|branch| branch.id == child.child_branch)
        .expect("failed child");
    assert_eq!(
        failed.outcome.as_deref(),
        Some("child agent execution failed")
    );
    assert_eq!(failed.model_invocations.len(), 1);
    assert_eq!(failed.model_invocations[0].request.as_ref(), Some(&request));
    assert_eq!(
        failed.model_invocations[0]
            .request
            .as_ref()
            .expect("persisted request")
            .active_components,
        request.active_components
    );
    assert_eq!(
        failed.model_invocations[0].provider_profile.as_deref(),
        Some("provider-profile")
    );
    assert_eq!(failed.model_invocations[0].provider_id, None);
    assert_eq!(
        failed.model_invocations[0].agent_profile.as_deref(),
        Some("profile.default@rev-3")
    );
    assert_eq!(
        failed.model_invocations[0].context_fingerprint.as_deref(),
        Some("context-fingerprint")
    );
    assert_eq!(
        failed.model_invocations[0].request_fingerprint.as_deref(),
        Some("request-fingerprint")
    );
}
