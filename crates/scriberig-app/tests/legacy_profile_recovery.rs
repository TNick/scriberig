//! Verifies explicit migration of a retired model profile into semantic profile state.

use std::sync::Arc;

use rpi_ai::{Api, Model, ThinkingLevel, ThinkingLevelMap};
use scriberig_app::{
    AgentConsoleBackend, AgentProfileDescriptor, AgentProfileExecutionPreference,
    AgentProfileProvider, AgentProfileRevisionRef, AgentProfileService, AgentProfileStoreResult,
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, Capability, CapabilityAction, CapabilityConstraints, CapabilityId,
    CapabilityScope, ContextActivationRule, ContextComponentDescriptor, ContextComponentKind,
    ContextComponentRevisionRef, LegacyExecutionMappingRequest, LegacyProfileMappingRequest,
};
use scriberig_core::{EncodingVersion, EntityId, ObjectType, RootName};
use scriberig_session::{Branch, Session, SessionExecutionSelection, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store::Revision;
use scriberig_store_memory::MemoryStore;

/// Backend fixture that rejects unrelated universal operations.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Returns unsupported-operation for requests outside the recovery test.
    ///
    /// # Parameters
    /// - `self`: Test backend.
    /// - `_context`: Unused test context.
    /// - `_request`: Unused semantic request.
    ///
    /// # Returns
    /// Never returns a successful result.
    ///
    /// # Errors
    /// Always reports the operation as unsupported.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "legacy profile fixture".to_owned(),
        })
    }
}

/// Builds a profile with explicit core behavior for the imported identity.
///
/// # Returns
/// A provider-neutral profile template for the retired profile identity.
fn migration_profile() -> AgentProfileDescriptor {
    AgentProfileDescriptor {
        reference: AgentProfileRevisionRef {
            identity: "retired-profile".to_owned(),
            revision: String::new(),
        },
        name: "Retired profile migration".to_owned(),
        components: vec![ContextComponentDescriptor {
            reference: ContextComponentRevisionRef {
                identity: "migration.core".to_owned(),
                revision: String::new(),
            },
            kind: ContextComponentKind::CoreInvariant,
            content: "Use semantic operations and preserve branch isolation.".to_owned(),
            activation: ContextActivationRule::Always,
            priority: 0,
        }],
        preferred_execution: None,
    }
}

/// Derives the stable semantic entity for the imported profile identity.
///
/// # Returns
/// The entity used to scope profile mutation capabilities.
fn profile_entity() -> EntityId {
    let kind = ObjectType::new("agent.profile.identity").expect("profile identity type");
    let revision = Revision::new(
        EncodingVersion::V1,
        kind,
        b"agent.profile\0retired-profile".to_vec(),
    )
    .expect("profile identity revision");
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&revision.id().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Creates a caller authorized to recover imported session and profile data.
///
/// # Returns
/// A context with session recovery and profile inspect/propose/promote grants.
fn context() -> ApplicationContext {
    let profile_grants = [
        CapabilityAction::Inspect,
        CapabilityAction::Custom("agent_profile.propose".to_owned()),
        CapabilityAction::Custom("agent_profile.promote".to_owned()),
    ]
    .into_iter()
    .map(|action| Capability {
        scope: CapabilityScope::Entity(profile_entity()),
        actions: [action].into_iter().collect(),
        constraints: CapabilityConstraints::unrestricted(),
    });
    let mut context = ApplicationContext {
        capabilities: scriberig_app::CapabilityContext::with_grants(profile_grants),
        ..ApplicationContext::empty()
    };
    context
        .capabilities
        .granted
        .push(CapabilityId::new("session_model_selection").expect("session capability"));
    context
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "This end-to-end regression exercises profile promotion, exact legacy record resolution, and persisted session selection in one SQLite lifecycle."
)]
/// Proposes a semantic profile only from an exact record and active catalog selection.
fn legacy_profile_mapping_promotes_and_resolves_exact_legacy_selection() {
    let database = std::env::temp_dir().join(format!(
        "scriberig-app-legacy-profile-{}.sqlite",
        EntityId::new()
    ));
    let session_store = Arc::new(SqliteSessionStore::open(&database).expect("open session store"));
    let session_id = EntityId::new();
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(EntityId::new(), "root").expect("root branch")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection {
            revision: 4,
            ..SessionExecutionSelection::default()
        },
    };
    session_store.save(&session).expect("save session");
    let mut legacy = serde_json::to_value(&session).expect("serialize session");
    legacy["execution"] = serde_json::json!({
        "revision": 4,
        "model_profile": "retired-profile",
        "model": "retired-model",
        "thinking_level": "deep",
        "history": []
    });
    session_store
        .write_legacy_payload_for_test(
            session_id,
            &serde_json::to_vec(&legacy).expect("encode legacy row"),
        )
        .expect("install legacy selection");

    let root = RootName::new("agent-profiles").expect("profile root");
    let profile_store = Arc::new(MemoryStore::new());
    let profile_service = AgentProfileService::new(Arc::clone(&profile_store), root);
    let base = profile_service
        .current_snapshot()
        .expect("empty profile base");
    let profile_provider: Arc<dyn AgentProfileProvider> = Arc::new(profile_service);
    let mut model = Model::new(
        "native-current",
        "Current model",
        Api::AnthropicMessages,
        "provider-current",
        "https://example.test",
    );
    model.thinking_level_map = Some(ThinkingLevelMap::from([
        (ThinkingLevel::Low, Some("low".to_owned())),
        (ThinkingLevel::Medium, Some("medium".to_owned())),
    ]));
    let application = scriberig_app::ApplicationService::new(Fixture)
        .with_session_store(session_store.clone())
        .with_agent_profile_provider(profile_provider)
        .with_upstream_models(vec![model]);
    let context = context();
    let request = LegacyProfileMappingRequest {
        session: session_id,
        expected_revision: 4,
        legacy_path: "/execution".to_owned(),
        base_profile_snapshot: base,
        profile: migration_profile(),
        provider_id: "provider-current".to_owned(),
        model_id: "native-current".to_owned(),
        thinking_level: Some("low".to_owned()),
    };

    let mut invalid = request.clone();
    invalid.profile.reference.identity = "different-profile".to_owned();
    assert!(
        application
            .propose_legacy_profile_mapping(&context, invalid)
            .is_err()
    );
    let mut stale = request.clone();
    stale.expected_revision = 3;
    assert!(matches!(
        application.propose_legacy_profile_mapping(&context, stale),
        Err(ApplicationError::Conflict { .. })
    ));
    let mut unknown_model = request.clone();
    unknown_model.model_id = "missing-model".to_owned();
    assert!(
        application
            .propose_legacy_profile_mapping(&context, unknown_model)
            .is_err()
    );
    assert_eq!(
        application
            .resolve_agent_profile(&context, "retired-profile")
            .expect("profile remains absent"),
        None
    );

    let candidate = application
        .propose_legacy_profile_mapping(&context, request)
        .expect("propose explicit legacy mapping");
    assert_eq!(candidate.profile_identity(), "retired-profile");
    assert_eq!(
        application
            .resolve_agent_profile(&context, "retired-profile")
            .expect("candidate leaves live root untouched"),
        None
    );
    assert_eq!(
        application
            .promote_agent_profile_candidate(&context, &candidate)
            .expect("promote authorized profile candidate"),
        AgentProfileStoreResult::Promoted {
            snapshot: candidate.snapshot()
        }
    );
    let promoted = application
        .resolve_agent_profile(&context, "retired-profile")
        .expect("resolve migrated profile")
        .expect("profile exists after promotion");
    assert_eq!(
        promoted.preferred_execution,
        Some(AgentProfileExecutionPreference {
            provider_id: "provider-current".to_owned(),
            model_id: "native-current".to_owned(),
            thinking_level: Some("low".to_owned()),
        })
    );
    assert_eq!(
        promoted.components[0].content,
        migration_profile().components[0].content
    );
    assert_eq!(
        session_store
            .legacy_execution_records(session_id)
            .expect("legacy record remains until selection migration")[0]
            .value["model_profile"],
        "retired-profile"
    );
    let selection = application
        .map_legacy_execution_selection(
            &context,
            &LegacyExecutionMappingRequest {
                session: session_id,
                expected_revision: 4,
                legacy_path: "/execution".to_owned(),
                provider_id: "provider-current".to_owned(),
                model_id: "native-current".to_owned(),
                thinking_level: Some("low".to_owned()),
            },
        )
        .expect("resolve exact legacy session selection");
    assert_eq!(selection.revision, 5);
    assert_eq!(selection.provider_id.as_deref(), Some("provider-current"));
    assert_eq!(selection.model_id.as_deref(), Some("native-current"));
    assert_eq!(selection.thinking_level.as_deref(), Some("low"));
    assert_eq!(
        session_store
            .legacy_execution_records(session_id)
            .expect("legacy record removal")
            .len(),
        0
    );
    drop(application);
    drop(session_store);
    let _ = std::fs::remove_file(database);
}
