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
        (
            "local".to_owned(),
            "runtime-default".to_owned(),
            None,
            "local".to_owned(),
        )
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
