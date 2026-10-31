//! Verifies artifact inspection is bounded and read-only for agent callers.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_agent_core::Tool as _;
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, Capability,
    CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityId, CapabilityScope,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;

/// Ensures artifact reads require both tool authority and project Inspect scope.
#[test]
fn artifact_agent_tool_returns_bounded_metadata_without_source_bytes() {
    let root = std::env::temp_dir().join(format!("scriberig-agent-artifacts-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("create project");
    fs::write(root.join("private.txt"), b"private source content").expect("write source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let snapshot = projects
        .import(scan.project.id)
        .expect("import project")
        .snapshot;
    let service = ApplicationService::new(ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    ))
    .with_project_service(projects);
    let tool = LocalAgentTool::new(&service);
    let arguments = serde_json::json!({"project": scan.project.id.to_string(), "limit": 1});

    assert!(
        tool.invoke_named(
            &ApplicationContext::default(),
            "artifact.inspect",
            &arguments
        )
        .is_err()
    );
    let broad_project_context = ApplicationContext {
        capabilities: CapabilityContext {
            granted: vec![CapabilityId::new("artifact.inspect").expect("tool capability")],
            grants: vec![Capability {
                scope: CapabilityScope::Project,
                actions: BTreeSet::from([CapabilityAction::Inspect]),
                constraints: CapabilityConstraints::unrestricted(),
            }],
        },
        snapshot: Some(snapshot),
        ..ApplicationContext::default()
    };
    assert!(
        tool.invoke_named(&broad_project_context, "artifact.inspect", &arguments)
            .is_err()
    );
    let context = authorized_context(scan.project.id, snapshot);
    let result = tool
        .invoke_named(&context, "artifact.inspect", &arguments)
        .expect("authorized artifact page");

    assert_eq!(result.object_type.as_deref(), Some("artifact.page"));
    assert_eq!(result.handles.len(), 1);
    let summary = result.summary.expect("bounded manifest summary");
    assert!(summary.contains("private.txt"));
    assert!(!summary.contains("private source content"));
    assert!(
        tool.invoke_named(
            &context,
            "artifact.inspect",
            &serde_json::json!({"project": scan.project.id.to_string(), "limit": 101}),
        )
        .is_err()
    );

    drop(tool);
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Builds a context with the tool capability and exact project Inspect grant.
///
/// - `project`: Project whose bounded artifact catalog is being inspected.
/// - `snapshot`: Exact project snapshot authorized for this agent branch.
///
/// Context accepted by the registered artifact read tool.
///
/// # Returns
/// A trusted capability context for the project.
fn authorized_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext {
            granted: vec![CapabilityId::new("artifact.inspect").expect("tool capability")],
            grants: vec![
                Capability {
                    scope: CapabilityScope::Project,
                    actions: BTreeSet::from([CapabilityAction::Custom("artifact.inspect".into())]),
                    constraints: CapabilityConstraints::unrestricted(),
                },
                Capability {
                    scope: CapabilityScope::Custom(format!("project:{project}")),
                    actions: BTreeSet::from([CapabilityAction::Inspect]),
                    constraints: CapabilityConstraints {
                        snapshot: Some(snapshot),
                        ..CapabilityConstraints::unrestricted()
                    },
                },
            ],
        },
        snapshot: Some(snapshot),
        ..ApplicationContext::default()
    }
}
