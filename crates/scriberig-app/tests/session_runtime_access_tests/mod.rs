//! Verifies session access operations enforce application capabilities.
use super::support::{Fixture, session_context};
use scriberig_app::{
    AgentConsoleBackend, ApplicationContext, ApplicationError, ApplicationService, SessionBackend,
    SessionRequest,
};
use scriberig_core::EntityId;

/// Verifies model selection fails without the model library capability.
#[test]
fn model_library_read_requires_agent_configuration_capability() {
    let service = ApplicationService::new(Fixture);

    let error = service
        .agent_model_library(&ApplicationContext::empty())
        .expect_err("model library must be authorized");

    assert_eq!(error.category(), "permission_denied");
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

/// Verifies authorized session reads continue to expose the durable session.
#[test]
fn session_read_with_capability_succeeds() {
    let service = ApplicationService::new(Fixture);
    let session = EntityId::from_bytes([1; 16]);
    service
        .start_agent(&session_context(), session, "visible")
        .expect("authorized session creation");

    assert_eq!(
        service
            .read_session(&session_context(), &SessionRequest { session })
            .expect("authorized session read")
            .branches
            .len(),
        1
    );
}
