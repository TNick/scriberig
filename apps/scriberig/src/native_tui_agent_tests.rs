//! Verifies exact project scope survives native agent continuation.

use std::sync::Arc;

use scriberig_app::{
    AgentCommandRequest, ApplicationBackendComposition, ApplicationContext, ApplicationService,
    CapabilityContext, CapabilityId, StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store_sqlite::SqliteStore;

use super::previous_workspace_selection;

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
