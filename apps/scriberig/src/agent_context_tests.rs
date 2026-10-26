//! Tests persisted root context compilation for the integrated application.

use std::sync::Arc;

use scriberig_app::{
    AgentProfileService, AgentProfileStoreResult, ApplicationBackendComposition,
    ApplicationContext, ApplicationService, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityId, CapabilityScope, SessionBackend, SessionRequest,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::{EntityId, RootName};
use scriberig_session::SessionStore;
use scriberig_store_sqlite::SqliteStore;

/// Compiles a persisted parentless root without requiring project scope.
#[test]
fn persisted_root_context_has_core_without_handoff() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = Arc::new(
        SqliteStore::open(temporary.path().join("project.sqlite")).expect("project store"),
    );
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(temporary.path().join("session.sqlite"))
            .expect("session store"),
    );
    let session_repository: Arc<dyn SessionStore> = session_store;
    let service = ApplicationService::new(backend).with_session_store(session_repository);
    let session_id = EntityId::from_bytes([191; 16]);
    let prompt = "inspect the root context path";
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session tree capability")
        ]),
        ..ApplicationContext::empty()
    };
    let handle = service
        .start_local_agent(&scriberig_app::AgentCommandRequest {
            session: session_id,
            branch: None,
            anchor: None,
            prompt: prompt.to_owned(),
            selection: None,
        })
        .expect("start root agent");

    let session = SessionBackend::read_session(
        &service,
        &context,
        &SessionRequest {
            session: session_id,
        },
    )
    .expect("read persisted session");
    let root = session
        .branches
        .iter()
        .find(|branch| branch.id == handle.root_branch)
        .expect("persisted root branch");
    assert!(root.parent.is_none());

    let compiled = super::compile_branch_context(
        &service,
        &context,
        session_id,
        handle.root_branch,
        prompt,
        None,
    )
    .expect("compile root context");

    assert!(!compiled.canonical.contains("semantic_project_scope"));
    assert!(
        compiled
            .canonical
            .contains("scriberig.context-component.core-invariants@")
    );
    assert!(
        !compiled
            .canonical
            .contains("scriberig.context-component.non-root-handoff@")
    );
    assert_eq!(compiled.canonical.matches(prompt).count(), 1);
}

#[expect(
    clippy::too_many_lines,
    reason = "integration regression checks persisted context end to end"
)]
/// Uses an exact durable profile revision in the normal branch-context compiler.
#[test]
fn persisted_profile_revision_drives_normal_branch_context() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = Arc::new(
        SqliteStore::open(temporary.path().join("profile-context.sqlite")).expect("project store"),
    );
    let profile_service = AgentProfileService::new(
        Arc::clone(&store),
        RootName::new("agent-profiles").expect("profile root"),
    );
    let mut profile = scriberig_agent_runtime::profile::bootstrap_default_agent_profile();
    let bootstrap_reference = profile.reference.clone();
    profile.components[0].content = "persisted profile marker".to_owned();
    let profile_entity = profile.reference.identity.clone();
    let profile_id = profile_entity;
    let base = profile_service.current_snapshot().expect("profile base");
    let profile_candidate = profile_service
        .propose_profile(
            &ApplicationContext {
                capabilities: CapabilityContext::with_grants([Capability {
                    scope: CapabilityScope::Project,
                    actions: [CapabilityAction::Custom("agent_profile.propose".to_owned())]
                        .into_iter()
                        .collect(),
                    constraints: CapabilityConstraints::unrestricted(),
                }]),
                ..ApplicationContext::empty()
            },
            base,
            profile,
        )
        .expect("profile candidate");
    let promotion_context = ApplicationContext {
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: [CapabilityAction::Custom("agent_profile.promote".to_owned())]
                .into_iter()
                .collect(),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::empty()
    };
    assert!(matches!(
        profile_service
            .promote_candidate(&promotion_context, &profile_candidate)
            .expect("profile promotion"),
        AgentProfileStoreResult::Promoted { .. }
    ));
    assert_eq!(
        profile_candidate.profile_identity(),
        bootstrap_reference.identity
    );
    assert_ne!(
        profile_candidate.revision().to_string(),
        bootstrap_reference.revision,
        "semantic storage must assign its own exact revision rather than aliasing the bootstrap revision"
    );

    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(temporary.path().join("session.sqlite"))
            .expect("session store"),
    );
    let service = ApplicationService::new(backend)
        .with_session_store(session_store)
        .with_agent_profile_provider(Arc::new(profile_service));
    let session_id = EntityId::from_bytes([192; 16]);
    let prompt = "use the stored profile";
    let handle = service
        .start_local_agent(&scriberig_app::AgentCommandRequest {
            session: session_id,
            branch: None,
            anchor: None,
            prompt: prompt.to_owned(),
            selection: None,
        })
        .expect("start root agent");
    let context = ApplicationContext {
        capabilities: CapabilityContext {
            granted: [CapabilityId::new("session_branch_tree").expect("session read grant")]
                .into_iter()
                .collect(),
            grants: [Capability {
                scope: CapabilityScope::Project,
                actions: [CapabilityAction::Inspect].into_iter().collect(),
                constraints: CapabilityConstraints::unrestricted(),
            }]
            .into_iter()
            .collect(),
        },
        ..ApplicationContext::empty()
    };
    let compiled = super::compile_branch_context(
        &service,
        &context,
        session_id,
        handle.root_branch,
        prompt,
        None,
    )
    .expect("compile with persisted profile");
    let captured_profile = service
        .resolve_agent_profile(
            &context,
            scriberig_agent_runtime::profile::DEFAULT_AGENT_PROFILE_IDENTITY,
        )
        .expect("resolve exact profile")
        .expect("persisted profile");
    let compiled_with_captured_profile = super::compile_branch_context_with_profile(
        &service,
        &context,
        session_id,
        handle.root_branch,
        prompt,
        None,
        &captured_profile,
    )
    .expect("compile with captured invocation profile");
    assert_eq!(compiled.profile.identity, profile_id);
    assert_eq!(
        compiled.profile.revision,
        profile_candidate.revision().to_string()
    );
    assert!(compiled.canonical.contains("persisted profile marker"));
    assert_eq!(
        compiled_with_captured_profile.profile,
        captured_profile.reference
    );
}
