//! Verifies local HTTP artifact reads resolve trusted project Inspect grants.

use super::{HttpState, dispatch};
use crate::development::DevelopmentFixture;
use scriberig_core::{EntityId, SnapshotId};

/// Returns artifact summaries only for projects in the host-owned catalog.
#[test]
fn typed_artifact_route_returns_project_scoped_fidelity_summaries() {
    let root = std::env::temp_dir().join(format!("scriberig-http-artifacts-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    std::fs::write(
        root.join("readme.txt"),
        b"source bytes stay out of summaries",
    )
    .expect("write source");
    let database = root.with_extension("sqlite");
    let store = std::sync::Arc::new(
        scriberig_store_sqlite::SqliteStore::open(&database).expect("open SQLite store"),
    );
    let projects = scriberig_project::ProjectService::new(std::sync::Arc::clone(&store));
    let project = projects.scan(None, &root).expect("scan project").project.id;
    let service = scriberig_app::ApplicationService::new(
        scriberig_app::StoreInspectionBackend::new(std::sync::Arc::clone(&store)),
    )
    .with_project_service(projects);
    let state = HttpState {
        service,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: Some(std::sync::Arc::clone(&store)),
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let body = serde_json::to_string(&scriberig_protocol::ArtifactInspectRequest {
        project: project.to_string(),
        offset: 0,
        limit: 10,
    })
    .expect("encode artifact request");
    let response = dispatch(&state, "artifact_inspect", &body, "artifact-1".to_owned())
        .expect("HTTP typed artifact response");

    assert_eq!(response["correlation_id"], "artifact-1");
    assert_eq!(
        response["response"]["ArtifactInspect"]["project"],
        project.to_string()
    );
    assert_eq!(response["response"]["ArtifactInspect"]["total"], 1);
    assert_eq!(
        response["response"]["ArtifactInspect"]["artifacts"][0]["members"][0]["project_path"],
        "readme.txt"
    );
    assert_eq!(
        response["response"]["ArtifactInspect"]["artifacts"][0]["member_count"],
        1
    );
    let member_body = serde_json::to_string(&scriberig_protocol::ArtifactMembersRequest {
        project: project.to_string(),
        artifact: response["response"]["ArtifactInspect"]["artifacts"][0]["id"]
            .as_str()
            .expect("artifact ID")
            .to_owned(),
        offset: 0,
        limit: 1,
    })
    .expect("encode member request");
    let member_response = dispatch(
        &state,
        "artifact_members",
        &member_body,
        "artifact-members-1".to_owned(),
    )
    .expect("HTTP typed artifact-member response");
    assert_eq!(member_response["response"]["ArtifactMembers"]["total"], 1);
    assert_eq!(
        member_response["response"]["ArtifactMembers"]["members"][0]["project_path"],
        "readme.txt"
    );
    assert!(!response.to_string().contains("source bytes stay out"));

    let foreign = scriberig_core::ProjectId::fresh();
    let denied_body = serde_json::to_string(&scriberig_protocol::ArtifactInspectRequest {
        project: foreign.to_string(),
        offset: 0,
        limit: 10,
    })
    .expect("encode foreign request");
    let denied = dispatch(
        &state,
        "artifact_inspect",
        &denied_body,
        "artifact-2".to_owned(),
    )
    .expect("structured permission response");
    assert_eq!(denied["response"]["Error"]["code"], "PermissionDenied");

    drop(state);
    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}
