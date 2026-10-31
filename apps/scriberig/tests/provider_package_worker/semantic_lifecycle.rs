//! Verifies a model-visible plugin semantic operation reaches validation and promotion.

use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult, ApplyRequest,
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityId,
    CapabilityScope, CommitRequest, OperationKind, OperationSummary, ValidationStatus,
};
use scriberig_core::{EncodingVersion, EntityId, RootName, TypeId};
use scriberig_query::{QueryValue, Selection};
use scriberig_store::{Revision, RevisionStore, RootStore, SnapshotChange, SnapshotStore};
use scriberig_store_memory::MemoryStore;

use super::{allowlisted_provider, start_fixture_worker_with_policy};

#[path = "semantic_lifecycle/backend.rs"]
mod backend;
use backend::LifecycleOperationBackend;

#[expect(
    clippy::too_many_lines,
    reason = "integration scenario verifies the complete plugin-to-promotion lifecycle"
)]
/// Proves a model-visible package tool reaches the shared candidate workflow.
#[tokio::test]
async fn model_visible_plugin_semantic_call_reaches_apply_and_promotion() {
    let launcher = scriberig_agent_runtime::plugin_worker::sandboxed_launcher()
        .expect("verified sandbox launcher");
    let plugin = super::provider_plugin_artifact();
    let protected_data = tempfile::tempdir().expect("protected fixture directory");
    let protected_roots = vec![
        protected_data
            .path()
            .canonicalize()
            .expect("canonical root"),
    ];
    let entity = EntityId::from_bytes([42; 16]);
    let store = Arc::new(MemoryStore::new());
    let empty_snapshot = store.empty_snapshot();
    let before = Revision::new(
        EncodingVersion::V1,
        TypeId::new("fixture.semantic_entity").expect("fixture entity type"),
        serde_json::to_vec(&serde_json::json!({"name": "original"}))
            .expect("fixture payload serializes"),
    )
    .expect("initial semantic revision");
    let before_id = store.put(before).expect("retain initial revision");
    let snapshot = store
        .apply_changes(empty_snapshot, &[SnapshotChange::Set(entity, before_id)])
        .expect("seed initial snapshot");
    store
        .set_root(RootName::new("main").expect("main root"), snapshot)
        .expect("seed canonical root");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let service = scriberig_app::ApplicationService::new(LifecycleOperationBackend::new(
        Arc::clone(&store),
        Arc::clone(&requests),
    ));
    let operation_capability = CapabilityId::new("fixture.rename").expect("operation id");
    let application_context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext {
            granted: vec![CapabilityId::new("domain_operation").expect("dispatcher grant")],
            grants: vec![Capability {
                scope: CapabilityScope::Entity(entity),
                actions: [CapabilityAction::Custom(operation_capability.to_string())]
                    .into_iter()
                    .collect(),
                constraints: CapabilityConstraints {
                    snapshot: Some(snapshot),
                    operation: Some(OperationKind::DomainOperation),
                    ..CapabilityConstraints::unrestricted()
                },
            }],
        },
        ..ApplicationContext::empty()
    };
    let dispatcher = Arc::new(
        scriberig_agent_runtime::plugin_worker::ApplicationDomainDispatcher::new(
            Arc::new(service.clone()),
            application_context.clone(),
            None,
        ),
    );
    let worker = start_fixture_worker_with_policy(
        &plugin,
        &protected_roots,
        launcher,
        std::collections::BTreeSet::from(["scriberig_fixture_tool".to_owned()]),
        std::collections::BTreeSet::from(["domain_operation".to_owned()]),
        dispatcher,
    );
    let plugin_capability = "fixture-plugin";
    let branch_capabilities = CapabilityContext {
        granted: vec![CapabilityId::new(plugin_capability).expect("plugin capability")],
        grants: Vec::new(),
    };
    let branch = EntityId::from_bytes([43; 16]);
    let composition = scriberig_agent_runtime::plugin_worker::PluginToolBridge::new(
        Arc::clone(&worker),
        plugin_capability,
    )
    .expect("verified worker bridge")
    .compose(scriberig_agent_runtime::plugin_worker::PluginToolRequest {
        catalog: scriberig_agent_runtime::tools::ToolCatalog::new(Vec::new()),
        capabilities: branch_capabilities,
        branch,
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        evidence: Arc::new(|_| {}),
    })
    .expect("compose worker tool into branch catalog");
    assert_eq!(composition.tools().len(), 1);

    let providers = worker
        .provider_adapters(&allowlisted_provider())
        .expect("provider adapter from the same package worker");
    let provider = Arc::clone(&providers[0]);
    let model = provider.models()[0].clone();
    let tool = serde_json::json!({
        "hostCapability": {
            "name": "domain_operation",
            "arguments": {
                "capability": operation_capability.to_string(),
                "subject": Selection {
                    snapshot,
                    entity,
                    revision: None,
                    relation_path: Vec::new(),
                    projection_anchor: None,
                },
                "arguments": [["name", QueryValue::Text("renamed".to_owned())]]
            }
        }
    });
    let prompt = format!(
        "fixture semantic rename: {}",
        serde_json::to_string(&tool).expect("serialize exact host request")
    );
    let agent_context = rpi_agent::AgentContext {
        system_prompt: String::new(),
        messages: Vec::new(),
        tools: composition
            .tools()
            .iter()
            .cloned()
            .map(|tool| tool as Arc<dyn rpi_agent::AgentTool>)
            .collect(),
    };
    let config = rpi_agent::AgentLoopConfig {
        model: model.clone(),
        provider_continuation: None,
        on_provider_continuation: None,
        continuation_stream_fn: None,
        cancellable_continuation_stream_fn: None,
        continuation_extractor: None,
        convert_to_llm: rpi_agent::hooks::default_convert_to_llm_fn(),
        transform_context: None,
        get_api_key: None,
        should_stop_after_turn: None,
        prepare_next_turn: None,
        after_tool_results: None,
        get_steering_messages: None,
        get_follow_up_messages: None,
        before_tool_call: None,
        after_tool_call: None,
        tool_execution: rpi_agent::ToolExecutionMode::Sequential,
        thinking_level: rpi_ai::types::ThinkingLevel::Off,
        api_key: None,
        timeout: None,
        max_retries: Some(0),
        max_retry_delay: None,
        cache_retention: rpi_ai::provider::CacheRetention::default(),
        session_id: None,
        signal: tokio_util::sync::CancellationToken::new(),
    };
    let stream_provider = Arc::clone(&provider);
    let stream_model = model.clone();
    let stream = rpi_agent::stream_fn(move |_model, context, options| {
        rpi_ai::stream_provider_cancellable(
            Arc::clone(&stream_provider),
            stream_model.clone(),
            context.clone(),
            options.clone(),
            None,
            None,
            None,
            false,
            tokio_util::sync::CancellationToken::new(),
        )
    });
    let (collector, events) = rpi_agent::CollectorEmitter::new();
    let messages = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        rpi_agent::run_agent_loop(
            vec![rpi_agent::AgentMessage::User(
                rpi_ai::types::UserMessage::new(prompt, 1),
            )],
            agent_context,
            config,
            Arc::new(collector),
            stream,
        ),
    )
    .await
    .expect("upstream plugin tool turn timed out")
    .expect("upstream model-visible plugin tool turn");
    let events = events.lock().expect("agent event lock").clone();
    assert!(events.iter().any(|event| matches!(
        event,
        rpi_agent::AgentEvent::ToolExecutionEnd {
            tool_name,
            is_error: false,
            ..
        } if tool_name == "scriberig_fixture_tool"
    )));
    assert!(
        serde_json::to_string(&messages)
            .expect("serialize upstream result messages")
            .contains("candidate:plugin-domain-operation"),
        "the model-visible tool result must contain the host-created candidate"
    );

    assert!(matches!(
        service.execute(
            &application_context,
            &ApplicationRequest::Commit(CommitRequest {
                base_snapshot: snapshot,
                branch: None,
                change: "candidate:plugin-domain-operation".to_owned(),
            }),
        ),
        Err(ApplicationError::InvalidRequest { .. })
    ));
    assert_eq!(
        store.read_root(&RootName::new("main").expect("main root")),
        Some(snapshot),
        "an unvalidated candidate must not be promoted"
    );
    let applied = service
        .execute(
            &application_context,
            &ApplicationRequest::Apply(ApplyRequest {
                base_snapshot: snapshot,
                branch: None,
                change: "candidate:plugin-domain-operation".to_owned(),
            }),
        )
        .expect("apply candidate through the shared application service");
    assert!(matches!(
        applied,
        ApplicationResult::Apply(OperationSummary {
            validation: Some(ValidationStatus::Passed),
            ..
        })
    ));
    assert_eq!(
        store.read_root(&RootName::new("main").expect("main root")),
        Some(snapshot)
    );
    let committed = service
        .execute(
            &application_context,
            &ApplicationRequest::Commit(CommitRequest {
                base_snapshot: snapshot,
                branch: None,
                change: "candidate:plugin-domain-operation".to_owned(),
            }),
        )
        .expect("promote validated candidate through the shared application service");
    let ApplicationResult::Commit(committed) = committed else {
        panic!("expected a committed semantic migration");
    };
    let promoted_snapshot = committed.snapshot.expect("promoted snapshot");
    assert_eq!(
        store.read_root(&RootName::new("main").expect("main root")),
        Some(promoted_snapshot)
    );
    let promoted_revision = store
        .resolve(promoted_snapshot, entity)
        .expect("resolve promoted entity")
        .expect("promoted entity exists");
    let promoted_payload: serde_json::Value = serde_json::from_slice(
        store
            .get(promoted_revision)
            .expect("read promoted revision")
            .expect("promoted revision exists")
            .payload(),
    )
    .expect("decode promoted payload");
    assert_eq!(promoted_payload["name"], "renamed");
    assert!(matches!(
        committed,
        OperationSummary { handle, .. } if handle == "candidate:plugin-domain-operation"
    ));
    let requests = requests.lock().expect("lifecycle request lock");
    assert!(matches!(
        requests.as_slice(),
        [
            ApplicationRequest::DomainOperation(_),
            ApplicationRequest::Commit(_),
            ApplicationRequest::Apply(_),
            ApplicationRequest::Commit(_)
        ]
    ));
    let ApplicationRequest::DomainOperation(operation) = &requests[0] else {
        unreachable!("first request is the plugin domain operation");
    };
    assert_eq!(operation.subject.entity, entity);
    assert_eq!(operation.subject.snapshot, snapshot);
    assert_eq!(
        operation.arguments,
        vec![("name".to_owned(), QueryValue::Text("renamed".to_owned()))]
    );
}
