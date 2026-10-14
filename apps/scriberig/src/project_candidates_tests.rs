//! Verifies project candidate tools through the application runtime boundary.

use std::fs;
use std::sync::Arc;

use scriberig_agent_core::Tool;
use scriberig_app::{
    ApplicationBackend, ApplicationBackendComposition, ApplicationContext, ApplicationService,
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_project::{ProjectSourceChange, ProjectSourceChangeKind};
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;

/// Exercises proposal, inspection, and validation without mutating live source state.
#[test]
fn candidate_tools_use_exact_shared_project_operations() {
    let root = std::env::temp_dir().join(format!("scriberig-agent-candidate-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("candidate project root");
    let source = b"base source\n";
    fs::write(root.join("notes.txt"), source).expect("source file");
    fs::create_dir_all(root.join("src")).expect("Rust source directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"candidate_fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("Cargo manifest");
    fs::write(
        root.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"candidate_fixture\"\nversion = \"0.1.0\"\n",
    )
    .expect("Cargo lockfile");
    fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").expect("Rust source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let project = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = project
        .scan(Some("candidate-tools".to_owned()), &root)
        .expect("scan");
    let imported = project.import(scan.project.id).expect("import");
    let actions = [
        "project_propose",
        "project_candidate",
        "project_validate_candidate",
        "project_resolve_dependencies",
    ]
    .into_iter()
    .map(|name| CapabilityAction::Custom(name.to_owned()))
    .collect::<std::collections::BTreeSet<_>>();
    let context = ApplicationContext {
        capabilities: CapabilityContext {
            granted: Vec::new(),
            grants: vec![Capability {
                scope: CapabilityScope::Project,
                actions,
                constraints: CapabilityConstraints::unrestricted(),
            }],
        },
        ..ApplicationContext::empty().with_snapshot(imported.snapshot)
    };
    let service = ApplicationService::new(ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    ));
    let tool = LocalAgentTool::new(&service).with_project_store(Arc::clone(&store));
    assert_candidate_tool_sequence(
        &tool,
        &context,
        &project,
        scan.project.id,
        imported.snapshot,
        &root,
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove candidate database");
    fs::remove_dir_all(root).expect("remove candidate fixture");
}

/// Verifies shared proposal, inspection, validation, and stale-base rejection.
///
/// # Parameters
/// - `tool`: Shared project-aware local agent tool adapter.
/// - `context`: Exact-base capability context.
/// - `project`: Imported project identity.
/// - `snapshot`: Immutable base snapshot.
/// - `root`: Live project root whose bytes must remain unchanged.
fn assert_candidate_tool_sequence<B: ApplicationBackend>(
    tool: &LocalAgentTool<'_, B>,
    context: &ApplicationContext,
    project_service: &scriberig_project::ProjectService,
    project: scriberig_core::ProjectId,
    snapshot: SnapshotId,
    root: &std::path::Path,
) {
    let source = b"base source\n";
    let proposed = tool
        .invoke_named(
            context,
            "project_propose",
            &serde_json::json!({
                "project": project.to_string(),
                "base_snapshot": snapshot.to_string(),
                "effects": [{"path":"notes.txt", "kind":"update", "content":"candidate source\n"}]
            }),
        )
        .expect("propose through shared project service");
    let candidate = proposed.handles[0]
        .strip_prefix("candidate:")
        .expect("candidate handle");
    let direct = project_service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "notes.txt".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"candidate source\n".to_vec()),
            }],
        )
        .expect("equivalent direct application proposal");
    assert_eq!(candidate, direct.id);
    assert_eq!(direct.base_snapshot, snapshot);
    assert_eq!(direct.effects.len(), 1);
    assert_eq!(direct.effects[0].path, "notes.txt");
    assert_eq!(direct.effects[0].old.as_deref(), Some(source.as_slice()));
    assert_eq!(
        direct.effects[0].new.as_deref(),
        Some(b"candidate source\n".as_slice())
    );
    let detail = tool
        .invoke_named(
            context,
            "project_candidate",
            &serde_json::json!({"candidate": candidate}),
        )
        .expect("inspect durable candidate");
    assert!(
        detail
            .summary
            .as_deref()
            .is_some_and(|value| value.contains("pending"))
    );
    assert!(
        detail
            .handles
            .iter()
            .any(|value| value.contains("update:notes.txt"))
    );
    assert_dependency_resolution(tool, context, project_service, candidate);
    assert_candidate_validation(tool, context, project_service, candidate);
    assert_eq!(
        fs::read(root.join("notes.txt")).expect("live source"),
        source
    );
    assert_eq!(
        project_service
            .workspace(project)
            .expect("workspace")
            .snapshot,
        snapshot
    );
    assert!(
        tool.invoke_named(
            context,
            "project_propose",
            &serde_json::json!({
                "project": project.to_string(),
                "base_snapshot": SnapshotId::new([0; 32]).to_string(),
                "effects": [{"path":"notes.txt", "kind":"update", "content":"stale"}]
            }),
        )
        .is_err()
    );
}

/// Acquires locked dependencies and proves acquisition alone cannot authorize Apply.
///
/// # Parameters
/// - `tool`: Shared project-aware local agent tool adapter.
/// - `context`: Capability context granting the specific dependency operation.
/// - `project_service`: Shared project candidate service.
/// - `candidate`: Stable candidate identity.
fn assert_dependency_resolution<B: ApplicationBackend>(
    tool: &LocalAgentTool<'_, B>,
    context: &ApplicationContext,
    project_service: &scriberig_project::ProjectService,
    candidate: &str,
) {
    let resolution = tool
        .invoke_named(
            context,
            "project_resolve_dependencies",
            &serde_json::json!({"candidate": candidate}),
        )
        .expect("resolve dependencies through bounded Cargo profile");
    assert!(
        resolution
            .handles
            .iter()
            .any(|value| value == "dependency_resolution:passed")
    );
    assert!(
        resolution
            .handles
            .iter()
            .any(|value| { value == "validation_profile:cargo-dependency-resolution" })
    );
    let resolved = project_service
        .candidate(candidate)
        .expect("durable resolution result")
        .expect("candidate persists");
    assert_eq!(
        resolved.validation_profile.as_deref(),
        Some("cargo-dependency-resolution")
    );
    assert_eq!(
        project_service
            .apply_candidate(&resolved)
            .expect_err("dependency acquisition is not code validation")
            .code,
        "validation_pending"
    );
}

/// Validates a candidate with a registered profile and verifies durable profile identity.
///
/// # Parameters
/// - `tool`: Shared project-aware local agent tool adapter.
/// - `context`: Exact-base capability context.
/// - `project_service`: Shared project candidate service.
/// - `candidate`: Stable candidate identity.
fn assert_candidate_validation<B: ApplicationBackend>(
    tool: &LocalAgentTool<'_, B>,
    context: &ApplicationContext,
    project_service: &scriberig_project::ProjectService,
    candidate: &str,
) {
    let validation = tool
        .invoke_named(
            context,
            "project_validate_candidate",
            &serde_json::json!({"candidate": candidate, "profile": "rust-workspace"}),
        )
        .expect("validate isolated candidate");
    assert!(
        validation
            .summary
            .as_deref()
            .is_some_and(|value| value.contains("passed"))
    );
    assert!(
        validation
            .handles
            .iter()
            .any(|value| value == "validation_profile:rust-workspace")
    );
    assert!(
        validation
            .handles
            .iter()
            .any(|value| { value == &format!("validation_id:{candidate}:rust-workspace") })
    );
    assert_eq!(
        project_service
            .candidate(candidate)
            .expect("durable candidate lookup")
            .expect("candidate remains persisted")
            .validation_profile
            .as_deref(),
        Some("rust-workspace")
    );
}
