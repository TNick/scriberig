//! Verifies nested child execution keeps its immediate durable parent.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::super::LocalChildExecutor;
use crate::native_provider::NativeProvider;
use scriberig_agent_runtime::cache::ModelCache;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderMetadata, ToolCall,
};
use scriberig_agent_runtime::{ChildExecutionScope, ChildExecutor, ChildRequest};
use scriberig_app::{
    AgentCommandRequest, ApplicationBackendComposition, ApplicationContext, ApplicationService,
    CapabilityContext, CapabilityId, SessionBackend, SessionRequest, StoreInspectionBackend,
    StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;

/// Persists a child and grandchild under their exact immediate parents.
#[test]
fn nested_child_uses_its_immediate_parent_and_non_root_context() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let project_store = Arc::new(
        SqliteStore::open(temporary.path().join("project.sqlite")).expect("project store"),
    );
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&project_store)),
        StoreOperationProvider::new(Arc::clone(&project_store)),
    );
    let session_store = Arc::new(
        SqliteSessionStore::open(temporary.path().join("session.sqlite")).expect("session store"),
    );
    let session_repository: Arc<dyn SessionStore> = session_store;
    let service = ApplicationService::new(backend).with_session_store(session_repository);
    let session = EntityId::new();
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session,
            branch: None,
            anchor: None,
            prompt: "root task".to_owned(),
            selection: None,
        })
        .expect("start root branch");
    let provider = FakeProvider::from_responses([
        Ok(delegation_response("grandchild")),
        Ok(final_response("grandchild result")),
        Ok(final_response("child result")),
    ]);
    let mut executor = LocalChildExecutor::new(
        &service,
        NativeProvider::Fake(provider.clone()),
        "fake-model".to_owned(),
        ModelCache::new(),
        Arc::clone(&project_store),
    );
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent.delegate").expect("delegation capability")
        ]),
        ..ApplicationContext::empty()
    };
    let result = executor
        .execute_child(
            &context,
            Some(ChildExecutionScope {
                session,
                parent_branch: root.root_branch,
                depth: 0,
                active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                semantic_scope: None,
            }),
            ChildRequest {
                purpose: "first child".to_owned(),
                prompt: "first child task".to_owned(),
                capabilities: vec!["agent.delegate".to_owned()],
                expected_result: "child result".to_owned(),
            },
            &AtomicBool::new(false),
        )
        .expect("execute nested children");
    let durable = SessionBackend::read_session(
        &service,
        &ApplicationContext {
            capabilities: CapabilityContext::new([
                CapabilityId::new("session_branch_tree").expect("session tree capability")
            ]),
            ..ApplicationContext::empty()
        },
        &SessionRequest { session },
    )
    .expect("read durable branch tree");
    let child = durable
        .branches
        .iter()
        .find(|branch| branch.id == result.branch.expect("first child branch"))
        .expect("first child persisted");
    let grandchild_id = child
        .children
        .first()
        .copied()
        .expect("grandchild linked to child");
    let grandchild = durable
        .branches
        .iter()
        .find(|branch| branch.id == grandchild_id)
        .expect("grandchild persisted");

    assert_eq!(child.parent, Some(root.root_branch));
    assert_eq!(grandchild.parent, Some(child.id));
    assert_eq!(grandchild.model_invocations.len(), 1);
    let projection = grandchild.model_invocations[0]
        .request
        .as_ref()
        .expect("grandchild captured request");
    assert_eq!(projection.active_components.len(), 2);
    assert!(projection.messages.iter().any(|message| {
        message
            .content
            .contains("one-paragraph summary for the parent")
    }));
    assert_eq!(provider.requests().len(), 3);
}

/// Creates a provider response that delegates one nested child.
///
/// - `purpose`: The delegated child purpose.
///
/// A provider response containing one delegation call.
/// # Returns
/// A tool-call response for one delegation.
fn delegation_response(purpose: &str) -> ModelResponse {
    ModelResponse {
        text: None,
        tool_calls: vec![ToolCall {
            id: format!("delegate-{purpose}"),
            name: "delegate".to_owned(),
            arguments: serde_json::json!({
                "purpose": purpose,
                "prompt": format!("{purpose} task"),
                "capabilities": [],
                "expected_result": format!("{purpose} result")
            }),
        }],
        finish_reason: FinishReason::ToolCall,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

/// Creates a provider response with final assistant text.
///
/// - `text`: Final assistant response.
///
/// A provider response containing final output.
/// # Returns
/// A completed assistant response containing `text`.
fn final_response(text: &str) -> ModelResponse {
    ModelResponse {
        text: Some(text.to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}
