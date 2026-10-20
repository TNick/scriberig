//! Verifies session access operations enforce application capabilities.
use super::support::{Fixture, session_context};
use scriberig_app::{
    AgentConsoleBackend, ApplicationContext, ApplicationError, ApplicationService, SessionBackend,
    SessionRequest,
};
use scriberig_core::EntityId;

/// Verifies session listing and removal require the explicit management grant.
#[test]
fn session_management_requires_explicit_capability() {
    let service = ApplicationService::new(Fixture);

    let list_error = service
        .list_sessions(&ApplicationContext::empty())
        .expect_err("session list must be authorized");
    let remove_error = service
        .remove_session(&ApplicationContext::empty(), EntityId::from_bytes([9; 16]))
        .expect_err("session removal must be authorized");

    assert!(matches!(
        list_error,
        ApplicationError::PermissionDenied { action, .. } if action == "session_management"
    ));
    assert!(matches!(
        remove_error,
        ApplicationError::PermissionDenied { action, .. } if action == "session_management"
    ));
}

/// Verifies idle session removal is durable while active work is protected.
#[test]
fn session_listing_and_removal_preserve_active_or_unselected_sessions() {
    use scriberig_session::{Branch, BranchStatus, Session, SessionExecutionSelection};

    let service = ApplicationService::new(Fixture);
    let context = ApplicationContext {
        capabilities: scriberig_app::CapabilityContext::new([
            scriberig_app::CapabilityId::new("session_management").expect("management grant"),
            scriberig_app::CapabilityId::new("agent_plan").expect("session write grant"),
        ]),
        ..ApplicationContext::empty()
    };
    let active_id = EntityId::from_bytes([81; 16]);
    let removable_id = EntityId::from_bytes([82; 16]);
    let mut active =
        Branch::new(EntityId::from_bytes([83; 16]), "active root").expect("active root");
    active.status = BranchStatus::Active;
    let mut removable = Branch::new(EntityId::from_bytes([84; 16]), "old root").expect("old root");
    removable.status = BranchStatus::Completed;
    for (id, branch) in [(active_id, active), (removable_id, removable)] {
        service
            .save_session(
                &context,
                &Session {
                    id,
                    branches: vec![branch],
                    goals: Vec::new(),
                    tasks: Vec::new(),
                    plans: Vec::new(),
                    execution: SessionExecutionSelection::default(),
                },
            )
            .expect("save session");
    }

    let listed = service.list_sessions(&context).expect("list sessions");
    assert_eq!(
        listed.iter().map(|session| session.id).collect::<Vec<_>>(),
        [active_id, removable_id]
    );
    assert!(service.remove_session(&context, active_id).is_err());
    service
        .remove_session(&context, removable_id)
        .expect("remove idle session");
    assert_eq!(
        service
            .list_sessions(&context)
            .expect("list after removal")
            .iter()
            .map(|session| session.id)
            .collect::<Vec<_>>(),
        [active_id]
    );
}

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
