//! Persists captured provider requests and checks their detail projection.

use std::sync::Arc;

use scriberig_agent_runtime::context::{
    BranchExecutionContext, BranchRole, ContextCompiler, ContextPolicy,
};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::profile::bootstrap_default_agent_profile;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderMetadata,
};
use scriberig_agent_runtime::{AgentMessage, ModelProvider};
use scriberig_app::{
    AgentCommandRequest, AgentConsoleBackend, AgentPresentationPreferences, ApplicationBackend,
    ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult,
    ApplicationService, CapabilityContext, CapabilityId, PromptVisibility, SessionBackend,
    SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_session::{ModelContextRevisionRef, ModelRequestProjection};
use scriberig_session_sqlite::SqliteSessionStore;

/// Backend fixture that leaves session ownership to the application service.
///
/// # Public members
/// None.
///
/// # Private members
/// None.
struct UnavailableBackend;

impl ApplicationBackend for UnavailableBackend {
    /// Rejects semantic operations unrelated to this persistence fixture.
    ///
    /// - `_context`: Application context unused by the fixture.
    /// - `_request`: Semantic request intentionally unsupported.
    ///
    /// Never returns an application result.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an unsupported-operation error.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "persistence fixture operation".to_owned(),
        })
    }
}

/// Persists and reloads one captured request, excluding provider credentials.
///
/// - `request`: Captured provider-neutral request projection.
/// - `secret`: Fake provider credential used by the HTTP fixture.
///
/// # Panics
/// Panics when session persistence or safe request-detail reading fails.
pub fn persist_and_assert_secret_absent(request: ModelRequestProjection, secret: &str) {
    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("SQLite session store"));
    let service = ApplicationService::new(UnavailableBackend).with_session_store(store);
    let session = EntityId::new();
    let handle = service
        .start_local_agent(&AgentCommandRequest {
            session,
            branch: None,
            anchor: None,
            prompt: "hello".to_owned(),
            selection: None,
        })
        .expect("start local agent");
    service
        .complete_local_agent_with_evidence(&handle, "done", Vec::new(), Vec::new(), vec![request])
        .expect("persist captured model request");

    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session read capability")
        ]),
        ..ApplicationContext::empty()
    };
    let loaded = SessionBackend::read_session(&service, &context, &SessionRequest { session })
        .expect("reload stored session");
    let encoded_session = serde_json::to_string(&loaded).expect("serialize loaded session");
    let invocation = loaded.branches[0].model_invocations[0].id;
    let detail = AgentConsoleBackend::model_request_detail(
        &service,
        &context,
        session,
        handle.root_branch,
        invocation,
    )
    .expect("read captured request detail");
    let encoded_detail = serde_json::to_string(&detail).expect("serialize request detail");

    assert!(!encoded_session.contains(secret));
    assert!(!encoded_detail.contains(secret));
    assert_eq!(detail.messages[0].content, "hello");
}

/// Persists live root and child captures with their exact compiled contexts.
#[test]
#[allow(clippy::too_many_lines)] // Keeps durable root/child provenance end to end.
fn root_and_child_provider_context_captures_survive_application_history() {
    let profile = bootstrap_default_agent_profile();
    let root_context = ContextCompiler::new(ContextPolicy::default())
        .with_profile(profile.clone())
        .compile(&BranchExecutionContext::new(
            "fixture-session",
            "fixture-root",
            "goal",
            "hello",
        ));
    let child_context = ContextCompiler::new(ContextPolicy::default())
        .with_profile(profile.clone())
        .compile(
            &BranchExecutionContext::new("fixture-session", "fixture-child", "goal", "hello")
                .with_branch_role(BranchRole::NonRoot {
                    parent: "fixture-root".to_owned(),
                }),
        );
    let response = || {
        Ok(ModelResponse {
            text: Some("done".to_owned()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            usage: None,
            metadata: ProviderMetadata::default(),
        })
    };
    let mut root_model =
        ProviderModel::new(FakeProvider::from_responses([response()]), "model-fixture")
            .with_provider_id("provider-fixture")
            .with_optional_provider_profile(Some("provider-profile-fixture".to_owned()))
            .with_thinking_level(Some("careful".to_owned()))
            .with_compiled_context(root_context.clone());

    root_model
        .decide(&[AgentMessage::User("hello".to_owned())])
        .expect("root provider decision");
    let root_capture = root_model
        .take_request_projections()
        .pop()
        .expect("live root request capture");
    let mut child_model =
        ProviderModel::new(FakeProvider::from_responses([response()]), "model-fixture")
            .with_provider_id("provider-fixture")
            .with_optional_provider_profile(Some("provider-profile-fixture".to_owned()))
            .with_thinking_level(Some("careful".to_owned()))
            .with_compiled_context(child_context.clone());
    child_model
        .decide(&[AgentMessage::User("hello".to_owned())])
        .expect("child provider decision");
    let child_capture = child_model
        .take_request_projections()
        .pop()
        .expect("live child request capture");

    assert_eq!(
        root_capture.profile,
        Some(ModelContextRevisionRef {
            identity: root_context.profile.identity.clone(),
            revision: root_context.profile.revision.clone(),
        })
    );
    assert_eq!(
        root_capture.active_components,
        root_context
            .active_components
            .iter()
            .map(|reference| ModelContextRevisionRef {
                identity: reference.identity.clone(),
                revision: reference.revision.clone(),
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(root_capture.active_components.len(), 1);
    assert_eq!(
        child_capture.profile,
        Some(ModelContextRevisionRef {
            identity: child_context.profile.identity.clone(),
            revision: child_context.profile.revision.clone(),
        })
    );
    assert_eq!(
        child_capture.active_components,
        child_context
            .active_components
            .iter()
            .map(|reference| ModelContextRevisionRef {
                identity: reference.identity.clone(),
                revision: reference.revision.clone(),
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(child_capture.active_components.len(), 2);
    assert!(root_capture.context_fingerprint.is_some());
    assert!(root_capture.request_fingerprint.is_some());
    assert_ne!(
        root_capture.context_fingerprint,
        child_capture.context_fingerprint
    );
    assert_ne!(
        root_capture.request_fingerprint,
        child_capture.request_fingerprint
    );

    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("SQLite session store"));
    let service = ApplicationService::new(UnavailableBackend).with_session_store(store);
    let session = EntityId::new();
    let handle = service
        .start_local_agent(&AgentCommandRequest {
            session,
            branch: None,
            anchor: None,
            prompt: "hello".to_owned(),
            selection: None,
        })
        .expect("start local agent");
    service
        .complete_local_agent_with_evidence(
            &handle,
            "done",
            Vec::new(),
            Vec::new(),
            vec![root_capture.clone()],
        )
        .expect("persist root provider capture through application history");
    let child_handle = service
        .start_local_child_agent(
            session,
            handle.root_branch,
            "fixture child",
            "goal",
            "hello",
        )
        .expect("start local child");
    service
        .complete_local_child_agent_with_evidence(
            &child_handle,
            "done",
            Vec::new(),
            Vec::new(),
            vec![child_capture.clone()],
        )
        .expect("persist child provider capture through application history");
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session read capability")
        ]),
        ..ApplicationContext::empty()
    };
    let loaded = SessionBackend::read_session(&service, &context, &SessionRequest { session })
        .expect("reload stored session");
    let root_branch = loaded
        .branches
        .iter()
        .find(|branch| branch.id == handle.root_branch)
        .expect("reloaded root branch");
    let child_branch = loaded
        .branches
        .iter()
        .find(|branch| branch.id == child_handle.child_branch)
        .expect("reloaded child branch");
    let root_invocation = &root_branch.model_invocations[0];
    let child_invocation = &child_branch.model_invocations[0];
    let expected_agent_profile = format!(
        "{}@{}",
        profile.reference.identity, profile.reference.revision
    );

    assert_eq!(root_invocation.request.as_ref(), Some(&root_capture));
    assert_eq!(child_invocation.request.as_ref(), Some(&child_capture));
    assert_eq!(
        root_invocation.agent_profile.as_deref(),
        Some(expected_agent_profile.as_str())
    );
    assert_eq!(
        root_invocation.context_fingerprint,
        root_capture.context_fingerprint
    );
    assert_eq!(
        root_invocation.request_fingerprint,
        root_capture.request_fingerprint
    );
    assert_eq!(
        child_invocation.context_fingerprint,
        child_capture.context_fingerprint
    );
    assert_eq!(
        child_invocation.request_fingerprint,
        child_capture.request_fingerprint
    );

    let mut later_profile = profile.clone();
    later_profile.reference.revision = "profile-revision-after-invocation".to_owned();
    later_profile.components[0].reference.revision =
        "component-revision-after-invocation".to_owned();
    let later_context = ContextCompiler::new(ContextPolicy::default())
        .with_profile(later_profile)
        .compile(&BranchExecutionContext::new(
            "fixture-session",
            "fixture-root",
            "goal",
            "hello",
        ));

    assert_ne!(
        later_context.profile.revision,
        root_capture
            .profile
            .as_ref()
            .expect("captured profile")
            .revision
    );
    assert_eq!(root_invocation.request.as_ref(), Some(&root_capture));
    assert_eq!(
        root_invocation.agent_profile.as_deref(),
        Some(expected_agent_profile.as_str())
    );

    let presentation_context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session read capability"),
            CapabilityId::new("agent_model_library").expect("presentation preference access"),
        ]),
        user_preferences_principal: Some("profile-visibility-fixture".to_owned()),
        ..ApplicationContext::empty()
    };
    let hidden_preferences = AgentConsoleBackend::set_agent_presentation_preferences(
        &service,
        &presentation_context,
        0,
        AgentPresentationPreferences::default(),
    )
    .expect("set hidden prompt preference");
    let hidden_detail = AgentConsoleBackend::model_request_detail(
        &service,
        &presentation_context,
        session,
        handle.root_branch,
        root_invocation.id,
    )
    .expect("read captured request with hidden prompt preference");
    AgentConsoleBackend::set_agent_presentation_preferences(
        &service,
        &presentation_context,
        hidden_preferences.revision,
        AgentPresentationPreferences {
            prompt: PromptVisibility::Shown,
            ..AgentPresentationPreferences::default()
        },
    )
    .expect("show prompt preference");
    let shown_detail = AgentConsoleBackend::model_request_detail(
        &service,
        &presentation_context,
        session,
        handle.root_branch,
        root_invocation.id,
    )
    .expect("read captured request with shown prompt preference");

    assert_eq!(hidden_detail, shown_detail);
    assert_eq!(shown_detail, root_capture);
}
