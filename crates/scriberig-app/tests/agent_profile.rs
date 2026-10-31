//! Verifies semantic profile revisions, candidate promotion, and exact historical reads.

use std::collections::BTreeSet;

use scriberig_app::{
    AgentProfileDescriptor, AgentProfileProvider, AgentProfileService, AgentProfileStoreResult,
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, Capability, CapabilityAction, CapabilityConstraints, CapabilityScope,
    ContextActivationRule, ContextComponentDescriptor, ContextComponentKind,
    ContextComponentRevisionRef,
};
use scriberig_core::{EncodingVersion, ObjectType, RootName};
use scriberig_store::Revision;
use scriberig_store_memory::MemoryStore;

/// Minimal backend for exercising profile operations through the app facade.
struct ProfileFacadeBackend;

impl ApplicationBackend for ProfileFacadeBackend {
    /// Rejects unrelated semantic application requests.
    ///
    /// # Parameters
    /// - `self`: Test backend.
    /// - `_context`: Caller context not used by this fixture.
    /// - `_request`: Request outside the profile operation under test.
    ///
    /// # Returns
    /// Never returns a successful operation result.
    ///
    /// # Errors
    /// Always reports the operation as unsupported.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "profile facade fixture".to_owned(),
        })
    }
}

/// Grants one profile entity's inspect, propose, and promotion operations.
///
/// # Parameters
/// - `entity`: Profile entity receiving the grants.
///
/// # Returns
/// A context with inspect, propose, and promotion authority for the profile.
fn authorized_context(entity: scriberig_core::EntityId) -> ApplicationContext {
    let grants = [
        CapabilityAction::Inspect,
        CapabilityAction::Custom("agent_profile.propose".to_owned()),
        CapabilityAction::Custom("agent_profile.promote".to_owned()),
    ]
    .into_iter()
    .map(|action| Capability {
        scope: CapabilityScope::Entity(entity),
        actions: BTreeSet::from([action]),
        constraints: CapabilityConstraints::unrestricted(),
    });
    ApplicationContext {
        capabilities: scriberig_app::CapabilityContext::with_grants(grants),
        ..ApplicationContext::default()
    }
}

/// Builds a profile with one component whose content can be revised.
///
/// # Parameters
/// - `content`: Component content to store in the profile.
///
/// # Returns
/// A deterministic profile descriptor with one core component.
fn profile(content: &str) -> AgentProfileDescriptor {
    AgentProfileDescriptor {
        reference: scriberig_app::AgentProfileRevisionRef {
            identity: "profile.default".to_owned(),
            revision: String::new(),
        },
        name: "Default".to_owned(),
        components: vec![ContextComponentDescriptor {
            reference: ContextComponentRevisionRef {
                identity: "component.core".to_owned(),
                revision: String::new(),
            },
            kind: ContextComponentKind::CoreInvariant,
            content: content.to_owned(),
            activation: ContextActivationRule::Always,
            priority: 0,
        }],
        preferred_execution: None,
    }
}

/// Derives the deterministic profile entity identity used by the app store.
///
/// # Returns
/// The stable entity identity for `profile.default`.
fn profile_entity() -> scriberig_core::EntityId {
    let kind = ObjectType::new("agent.profile.identity").expect("type name");
    let revision = Revision::new(
        EncodingVersion::V1,
        kind,
        b"agent.profile\0profile.default".to_vec(),
    )
    .expect("identity revision");
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&revision.id().as_bytes()[..16]);
    scriberig_core::EntityId::from_bytes(bytes)
}

/// Persists profiles as independent exact component/profile revisions and rejects stale promotion.
#[test]
fn profile_candidates_preserve_history_and_conflict_on_stale_base() {
    let store = MemoryStore::new();
    let root = RootName::new("agent-profiles").expect("root name");
    let service = AgentProfileService::new(store, root);
    let base = service.current_snapshot().expect("empty base");
    let context = authorized_context(profile_entity());

    let first = service
        .propose_profile(&context, base, profile("core v1"))
        .expect("first candidate");
    assert_eq!(
        service
            .promote_candidate(&context, &first)
            .expect("first promotion"),
        AgentProfileStoreResult::Promoted {
            snapshot: first.snapshot()
        }
    );
    let first_inspection = service
        .inspect_profile(
            &context,
            first.snapshot(),
            "profile.default",
            first.revision(),
        )
        .expect("first exact profile");
    assert_eq!(first_inspection.profile.components[0].content, "core v1");

    let second = service
        .propose_profile(&context, first.snapshot(), profile("core v2"))
        .expect("second candidate");
    let concurrent = service
        .propose_profile(&context, first.snapshot(), profile("core v3"))
        .expect("concurrent candidate");
    assert_ne!(second.revision(), concurrent.revision());
    assert!(matches!(
        service
            .promote_candidate(&context, &concurrent)
            .expect("concurrent promotion"),
        AgentProfileStoreResult::Promoted { .. }
    ));
    assert_eq!(
        service
            .promote_candidate(&context, &second)
            .expect("stale promotion"),
        AgentProfileStoreResult::Stale {
            actual: Some(concurrent.snapshot())
        }
    );
    let rejected_candidate = service
        .inspect_profile(
            &context,
            second.snapshot(),
            "profile.default",
            second.revision(),
        )
        .expect("stale candidate remains inspectable");
    assert_eq!(rejected_candidate.profile.components[0].content, "core v2");
    let historic_component = service
        .inspect_profile(
            &context,
            first.snapshot(),
            "profile.default",
            first.revision(),
        )
        .expect("historical profile remains resolvable");
    assert_eq!(historic_component.profile.components[0].content, "core v1");
    assert_ne!(
        historic_component.profile.components[0].reference.revision,
        service
            .inspect_profile(
                &context,
                concurrent.snapshot(),
                "profile.default",
                concurrent.revision(),
            )
            .expect("latest profile")
            .profile
            .components[0]
            .reference
            .revision
    );
}

/// Rejects malformed profile changes before storing a candidate snapshot.
#[test]
fn profile_proposal_rejects_duplicate_component_identity() {
    let service = AgentProfileService::new(
        MemoryStore::new(),
        RootName::new("agent-profiles").expect("root name"),
    );
    let base = service.current_snapshot().expect("empty base");
    let mut malformed = profile("core");
    malformed.components.push(malformed.components[0].clone());
    let context = authorized_context(profile_entity());
    assert!(service.propose_profile(&context, base, malformed).is_err());
    assert_eq!(
        service
            .list_profiles(&context, base)
            .expect("no candidate was promoted")
            .len(),
        0
    );
}

/// Confirms profile and component revisions survive closing and reopening SQLite.
#[test]
fn promoted_profile_reopens_from_durable_universal_store() {
    let directory = tempfile::tempdir().expect("temporary profile database");
    let path = directory.path().join("profiles.sqlite");
    let root = RootName::new("agent-profiles").expect("root name");
    let (snapshot, revision) = {
        let store = scriberig_store_sqlite::SqliteStore::open(&path).expect("open store");
        let service = AgentProfileService::new(store, root.clone());
        let base = service.current_snapshot().expect("empty base");
        let context = authorized_context(profile_entity());
        let candidate = service
            .propose_profile(&context, base, profile("durable core"))
            .expect("profile candidate");
        service
            .promote_candidate(&context, &candidate)
            .expect("profile promotion");
        (candidate.snapshot(), candidate.revision())
    };
    let store = scriberig_store_sqlite::SqliteStore::open(&path).expect("reopen store");
    let service = AgentProfileService::new(store, root);
    let inspection = service
        .inspect_profile(
            &authorized_context(profile_entity()),
            snapshot,
            "profile.default",
            revision,
        )
        .expect("read promoted profile after restart");
    assert_eq!(inspection.profile.components[0].content, "durable core");
    assert_eq!(inspection.profile.reference.revision, revision.to_string());
}

/// Uses the shared application facade for profile candidate and promotion operations.
#[test]
fn profile_candidates_flow_through_application_facade() {
    let store = MemoryStore::new();
    let root = RootName::new("agent-profiles").expect("root name");
    let profile_service = AgentProfileService::new(store, root);
    let base = profile_service.current_snapshot().expect("empty base");
    let context = authorized_context(profile_entity());
    let provider: std::sync::Arc<dyn AgentProfileProvider> = std::sync::Arc::new(profile_service);
    let application = scriberig_app::ApplicationService::new(ProfileFacadeBackend)
        .with_agent_profile_provider(provider);
    assert_eq!(
        application
            .current_agent_profile_snapshot()
            .expect("profile base snapshot through application facade"),
        base
    );

    let candidate = application
        .propose_agent_profile(&context, base, profile("migrated core"))
        .expect("propose through application facade");
    assert_eq!(
        application
            .resolve_agent_profile(&context, "profile.default")
            .expect("resolve before promotion"),
        None,
        "candidate creation leaves the accepted profile unchanged"
    );
    assert_eq!(
        application
            .promote_agent_profile_candidate(&context, &candidate)
            .expect("promote through application facade"),
        AgentProfileStoreResult::Promoted {
            snapshot: candidate.snapshot()
        }
    );
    let accepted = application
        .resolve_agent_profile(&context, "profile.default")
        .expect("resolve promoted profile")
        .expect("accepted profile exists");
    assert_eq!(accepted.components[0].content, "migrated core");
    assert_eq!(
        accepted.reference.revision,
        candidate.revision().to_string()
    );
}
