//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies native runtime model resolution uses fallback without selection.
#[test]
fn native_runtime_model_resolution_uses_fallback_without_selection() {
    let resolved = resolve_selected_runtime_model(
        &running_invocation_upstream_models(),
        &SessionExecutionSelection::default(),
        "runtime-default",
    )
    .expect("resolve runtime fallback");

    assert_eq!(
        resolved,
        ("runtime-default".to_owned(), None, "local".to_owned())
    );
}

/// Keeps an active invocation's resolved settings after session selection changes.
#[test]
fn running_invocation_keeps_its_model_and_thinking_snapshot() {
    let active_selection = SessionExecutionSelection {
        revision: 1,
        provider_id: Some("local".to_owned()),
        model_id: Some("wire-a".to_owned()),
        thinking_level: Some("medium".to_owned()),
        history: Vec::new(),
    };
    let upstream_models = running_invocation_upstream_models();
    let active_invocation =
        resolve_selected_runtime_model(&upstream_models, &active_selection, "runtime-default")
            .expect("resolve active invocation");
    let provider = FakeProvider::from_responses([
        Ok(ModelResponse {
            text: Some("active response".to_owned()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            usage: None,
            metadata: ProviderMetadata::default(),
        }),
        Ok(ModelResponse {
            text: Some("next response".to_owned()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            usage: None,
            metadata: ProviderMetadata::default(),
        }),
    ]);
    let mut active_model = ProviderModel::new(provider.clone(), active_invocation.0.clone())
        .with_thinking_level(active_invocation.1.clone());
    let next_selection = SessionExecutionSelection {
        revision: 2,
        provider_id: Some("local".to_owned()),
        model_id: Some("wire-b".to_owned()),
        thinking_level: Some("high".to_owned()),
        history: Vec::new(),
    };
    let next_invocation =
        resolve_selected_runtime_model(&upstream_models, &next_selection, "runtime-default")
            .expect("resolve next invocation");
    active_model
        .decide(&[AgentMessage::User("active prompt".to_owned())])
        .expect("active invocation response");
    let mut next_model = ProviderModel::new(provider.clone(), next_invocation.0.clone())
        .with_thinking_level(next_invocation.1.clone());
    next_model
        .decide(&[AgentMessage::User("next prompt".to_owned())])
        .expect("next invocation response");

    assert_eq!(
        active_invocation,
        (
            "wire-a".to_owned(),
            Some("medium".to_owned()),
            "local".to_owned()
        )
    );
    assert_eq!(
        next_invocation,
        (
            "wire-b".to_owned(),
            Some("high".to_owned()),
            "local".to_owned()
        )
    );
    assert_eq!(provider.requests()[0].model, "wire-a");
    assert_eq!(
        provider.requests()[0].thinking_level.as_deref(),
        Some("medium")
    );
    assert_eq!(provider.requests()[1].model, "wire-b");
    assert_eq!(
        provider.requests()[1].thinking_level.as_deref(),
        Some("high")
    );
}

/// Creates upstream models for the invocation snapshot fixture.
///
/// The two model configurations selected by the fixture.
/// # Returns
/// The operation result.
fn running_invocation_upstream_models() -> Vec<rpi_ai::Model> {
    vec![
        rpi_ai::Model::new(
            "wire-a",
            "Model A",
            rpi_ai::Api::AnthropicMessages,
            "local",
            "https://example.test",
        ),
        rpi_ai::Model::new(
            "wire-b",
            "Model B",
            rpi_ai::Api::AnthropicMessages,
            "local",
            "https://example.test",
        ),
        rpi_ai::Model::new(
            "runtime-default",
            "Runtime fallback",
            rpi_ai::Api::AnthropicMessages,
            "local",
            "https://example.test",
        ),
    ]
}

/// Recovers the exact project scope stored by a completed onboarding branch.
#[test]
fn continuation_uses_durable_project_snapshot_and_root_handles() {
    let root = std::env::temp_dir().join(format!("scriberig-continuation-{0}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("fixture root");
    let database = root.join("state.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let service = ApplicationService::new(backend);
    let session = EntityId::new();
    let project = ProjectId::fresh();
    let snapshot = SnapshotId::new([42; 32]);
    let root_entity = EntityId::new();
    let handle = service
        .start_local_agent(&AgentCommandRequest {
            session,
            branch: None,
            anchor: None,
            prompt: "onboard".to_owned(),
            selection: None,
        })
        .expect("start onboarding branch");
    service
        .complete_local_agent_with_handles(
            &handle,
            "ready",
            vec![
                format!("project:{project}"),
                format!("snapshot:{snapshot}"),
                format!("root:{root_entity}"),
            ],
        )
        .expect("complete onboarding branch");
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session capability")
        ]),
        ..ApplicationContext::empty()
    };

    let selection = previous_workspace_selection(&service, &context, session)
        .expect("read prior workspace")
        .expect("workspace selection");
    assert_eq!(selection.project, Some(project));
    assert_eq!(selection.snapshot, snapshot);
    assert_eq!(selection.root, root_entity);
    assert_eq!(selection.entity, root_entity);

    drop(service);
    drop(store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}
