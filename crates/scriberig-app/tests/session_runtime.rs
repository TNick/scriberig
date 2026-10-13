//! Verifies application-owned fallback session and agent state.

use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationRequest, ApplicationResult, ApplicationService, CapabilityContext, CapabilityId,
    SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;

/// Backend fixture that leaves session and agent ownership to the application.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal requests because this test targets session fallback.
    ///
    /// # Parameters
    /// - `_context`: Application context, unused by the fixture.
    /// - `_request`: Universal request, intentionally unsupported.
    ///
    /// # Returns
    /// Never returns a universal result.
    ///
    /// # Errors
    /// Always returns unsupported-operation.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture universal request".to_owned(),
        })
    }
}

/// Creates the session-tree capability used by authorized test callers.
///
/// # Returns
/// Application context granting the session-tree and agent capabilities.
fn session_context() -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("capability"),
            CapabilityId::new("agent_start").expect("capability"),
            CapabilityId::new("agent_continue").expect("capability"),
        ]),
        ..ApplicationContext::empty()
    }
}

/// Verifies local agent acceptance creates and updates a session branch.
#[test]
fn fallback_agent_acceptance_retains_session_activity() {
    let service = ApplicationService::new(Fixture);
    let session = EntityId::from_bytes([1; 16]);
    let context = session_context();
    let first = service
        .start_agent(&context, session, "first")
        .expect("first agent operation");
    let state = service
        .read_session(
            &session_context(),
            &scriberig_app::SessionRequest { session },
        )
        .expect("session state");
    assert_eq!(state.branches.len(), 1);
    assert_eq!(state.branches[0].turns.len(), 1);
    assert_eq!(
        state.branches[0].turns[0].human.as_ref().unwrap().text,
        "first"
    );
    assert_eq!(
        state.branches[0].turns[0].agent.as_ref().unwrap().text,
        "first"
    );
    assert!(state.branches[0].result.is_some());
    let first_record = service.operations().get(&first).unwrap();
    assert_eq!(
        first_record.status,
        scriberig_app::OperationStatus::Completed
    );
    assert_eq!(
        first_record.agent_result.as_ref().unwrap().final_output,
        "first"
    );
    assert_eq!(first_record.kind, scriberig_app::OperationKind::Agent);

    let branch = state.branches[0].id;
    let second = service
        .continue_agent(&context, session, branch, "second")
        .expect("continuation operation");
    assert_ne!(first, second);
    let state = service
        .read_session(
            &session_context(),
            &scriberig_app::SessionRequest { session },
        )
        .expect("continued session state");
    assert_eq!(state.branches.len(), 2);
    assert_eq!(state.branches[0].turns.len(), 1);
    assert_eq!(state.branches[1].parent, Some(branch));
    assert_eq!(state.branches[1].turns.len(), 1);
    assert_eq!(
        state.branches[1].turns[0]
            .human
            .as_ref()
            .expect("continuation input")
            .text,
        "second"
    );
}

/// Verifies authorized bootstrap execution can create a durable operation without project scope.
#[test]
fn bootstrap_agent_execution_starts_without_semantic_project_scope() {
    let service = ApplicationService::new(Fixture);
    let session = EntityId::from_bytes([9; 16]);
    let context = session_context();
    let handle = service
        .begin_agent_execution(
            &context,
            &AgentCommandRequest {
                session,
                branch: None,
                prompt: "Onboard the current project.".to_owned(),
                selection: None,
            },
        )
        .expect("bootstrap operation accepted without project scope");
    assert_eq!(handle.session, session);
    assert_eq!(
        service.operations().get(&handle.operation).unwrap().status,
        scriberig_app::OperationStatus::Running
    );
    let session = service
        .read_session(&session_context(), &SessionRequest { session })
        .expect("durable session");
    assert_eq!(session.branches.len(), 1);
    assert_eq!(
        session.branches[0].turns[0].human.as_ref().unwrap().text,
        "Onboard the current project."
    );
}

/// Verifies an agent command cannot bypass the application capability boundary.
#[test]
fn agent_submission_without_capability_is_denied() {
    let service = ApplicationService::new(Fixture);
    let error = service
        .start_agent(
            &ApplicationContext::empty(),
            EntityId::from_bytes([4; 16]),
            "denied",
        )
        .expect_err("agent capability must be required");

    assert!(matches!(
        error,
        ApplicationError::PermissionDenied { action, .. } if action == "agent_start"
    ));
}

/// Verifies session-tree reads cannot bypass the application capability boundary.
#[test]
fn session_read_without_capability_is_denied() {
    let service = ApplicationService::new(Fixture);
    let error = service
        .read_session(
            &ApplicationContext::empty(),
            &SessionRequest {
                session: EntityId::from_bytes([5; 16]),
            },
        )
        .expect_err("session capability must be required");

    assert!(matches!(
        error,
        ApplicationError::PermissionDenied { action, .. } if action == "session_branch_tree"
    ));
}

/// Verifies a local runtime can complete the shared operation with its output.
#[test]
fn local_agent_completion_uses_runtime_output() {
    let service = ApplicationService::new(Fixture);
    let session = EntityId::from_bytes([2; 16]);
    let handle = service
        .start_local_agent(&scriberig_app::AgentCommandRequest {
            session,
            branch: None,
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
        service.operations().get(&handle.operation).unwrap().status,
        scriberig_app::OperationStatus::Completed
    );
}

/// Verifies child operation startup creates an explicit session graph edge.
#[test]
fn local_child_agent_start_creates_parent_link() {
    let service = ApplicationService::new(Fixture);
    let request = AgentCommandRequest {
        session: EntityId::from_bytes([21; 16]),
        branch: None,
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
    assert_eq!(
        session
            .branches
            .iter()
            .find(|branch| branch.id == child.child_branch)
            .and_then(|branch| branch.parent),
        Some(root.root_branch)
    );

    let completed = service
        .complete_local_child_agent_with_handles(
            &child,
            "child output",
            vec!["project:prj-one".to_owned(), "snapshot:snp-base".to_owned()],
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
        child_branch
            .result
            .as_ref()
            .expect("durable result")
            .result_handles,
        vec!["project:prj-one", "snapshot:snp-base"]
    );
}

/// Verifies failed child execution is retained as an abandoned branch result.
#[test]
fn failed_local_child_agent_is_retained() {
    let service = ApplicationService::new(Fixture);
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([31; 16]),
            branch: None,
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
    assert_eq!(
        service.operations().get(&child.operation).unwrap().status,
        scriberig_app::OperationStatus::Failed
    );
}
