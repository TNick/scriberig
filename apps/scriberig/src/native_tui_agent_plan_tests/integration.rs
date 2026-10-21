//! Verifies native plan integration nodes use project candidate merge semantics.

use super::*;
use scriberig_agent_runtime::context::{
    CompactResultContext, ContextCompiler, ContextPolicy, ProjectSemanticScope,
};
use scriberig_agent_runtime::plan_context::PlanWorkerContext;
use scriberig_agent_runtime::scheduler::PlanWorkerExecutor;
use scriberig_project::{ProjectSourceChange, ProjectSourceChangeKind};
use scriberig_session::{ExpectedResult, PlanNode, PlanNodeStatus, WorkerClaim};

#[path = "integration/coordinator.rs"]
mod coordinator;

/// Merges candidates from exact dependency results without promoting the project.
#[test]
fn plan_integration_node_merges_disjoint_dependency_candidates() {
    let fixture = candidate_fixture();
    let project = scriberig_project::ProjectService::new(Arc::clone(&fixture.store));
    let left = project
        .propose_project_change(
            fixture.project,
            fixture.snapshot,
            vec![ProjectSourceChange {
                path: "notes.txt".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"left candidate\n".to_vec()),
            }],
        )
        .expect("left candidate");
    let right = project
        .propose_project_change(
            fixture.project,
            fixture.snapshot,
            vec![ProjectSourceChange {
                path: "Cargo.lock".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"version = 4\n# right candidate\n".to_vec()),
            }],
        )
        .expect("right candidate");
    let result = execute_integration(&fixture, [&left.id, &right.id])
        .expect("integration node should merge candidates");

    assert_eq!(
        result.result_kind.as_deref(),
        Some("project_candidate_merge")
    );
    let merged_handle = result
        .result_handles
        .iter()
        .find_map(|handle| handle.strip_prefix("candidate:"))
        .expect("merged candidate handle");
    let merged = project
        .candidate(merged_handle)
        .expect("read merged candidate")
        .expect("candidate exists");
    assert_eq!(merged.base_snapshot, fixture.snapshot);
    assert_eq!(merged.effects.len(), 2);
    assert!(
        result
            .result_handles
            .contains(&format!("candidate_parent:{}", left.id))
    );
    assert!(
        result
            .result_handles
            .contains(&format!("candidate_parent:{}", right.id))
    );
    assert_eq!(
        project
            .workspace(fixture.project)
            .expect("workspace")
            .snapshot,
        fixture.snapshot
    );
    assert_eq!(
        std::fs::read(fixture.root.join("notes.txt")).expect("source projection"),
        b"base notes\n"
    );
    drop(project);
    drop(fixture.store);
    std::fs::remove_dir_all(fixture.root.parent().expect("fixture parent"))
        .expect("remove candidate fixture");
}

/// Preserves same-target conflicts as structured plan-worker failures.
#[test]
fn plan_integration_node_reports_same_target_candidate_conflict() {
    let fixture = candidate_fixture();
    let project = scriberig_project::ProjectService::new(Arc::clone(&fixture.store));
    let left = project
        .propose_project_change(
            fixture.project,
            fixture.snapshot,
            vec![ProjectSourceChange {
                path: "notes.txt".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"left candidate\n".to_vec()),
            }],
        )
        .expect("left candidate");
    let right = project
        .propose_project_change(
            fixture.project,
            fixture.snapshot,
            vec![ProjectSourceChange {
                path: "notes.txt".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"right candidate\n".to_vec()),
            }],
        )
        .expect("right candidate");
    let error = execute_integration(&fixture, [&left.id, &right.id])
        .expect_err("same-target changes must conflict");

    assert!(error.starts_with("candidate_merge_conflict: "));
    assert_eq!(
        project
            .workspace(fixture.project)
            .expect("workspace")
            .snapshot,
        fixture.snapshot
    );
    drop(project);
    drop(fixture.store);
    std::fs::remove_dir_all(fixture.root.parent().expect("fixture parent"))
        .expect("remove candidate fixture");
}

/// Holds one isolated project with two candidate-capable source files.
struct CandidateFixture {
    /// Durable SQLite project and candidate storage.
    store: Arc<SqliteStore>,
    /// Imported project identity.
    project: scriberig_core::ProjectId,
    /// Exact immutable snapshot used by all proposals.
    snapshot: SnapshotId,
    /// Filesystem projection root used for non-mutation assertions.
    root: std::path::PathBuf,
}

/// Creates an imported project fixture for merge integration tests.
/// # Returns
/// The durable project fixture.
fn candidate_fixture() -> CandidateFixture {
    let parent = agentic_fixture_root("plan-candidates");
    let root = parent.join("project");
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"plan_candidate_fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    std::fs::write(root.join("Cargo.lock"), "version = 4\n").expect("lock file");
    std::fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").expect("Rust source");
    std::fs::write(root.join("notes.txt"), "base notes\n").expect("notes source");
    let store = Arc::new(SqliteStore::open(parent.join("project.sqlite")).expect("store"));
    let service = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = service
        .scan(Some("plan-candidate-fixture".to_owned()), &root)
        .expect("scan project");
    let imported = service.import(scan.project.id).expect("import project");
    CandidateFixture {
        store,
        project: scan.project.id,
        snapshot: imported.snapshot,
        root,
    }
}

/// Executes one synthetic integration node through the production worker.
///
/// - `fixture`: Exact project and semantic candidate store.
/// - `candidate_ids`: Two durable candidates made by dependency workers.
///
/// The durable integration branch result or a structured candidate error.
/// # Returns
/// The worker result.
/// # Errors
/// Returns an error when the candidate merge cannot be represented safely.
fn execute_integration(
    fixture: &CandidateFixture,
    candidate_ids: [&str; 2],
) -> Result<scriberig_session::BranchResult, String> {
    let store = Arc::clone(&fixture.store);
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let service = ApplicationService::new(backend);
    let session = EntityId::new();
    let plan = EntityId::new();
    let node_id = EntityId::new();
    let branch = EntityId::new();
    let claim_id = EntityId::new();
    let node = PlanNode {
        id: node_id,
        parent: None,
        title: "Integrate candidate changes".to_owned(),
        purpose: "Merge exact dependency candidates".to_owned(),
        expected_result: Some(ExpectedResult {
            kind: "project_candidate_merge".to_owned(),
            requires_handle: true,
        }),
        semantic_scope: vec![
            format!("project:{}", fixture.project),
            format!("snapshot:{}", fixture.snapshot),
        ],
        status: PlanNodeStatus::Running,
        branch: Some(branch),
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    };
    let claim = WorkerClaim {
        id: claim_id,
        node: node_id,
        branch,
        owner: "plan-worker".to_owned(),
        generation: 1,
        assignment: None,
    };
    let mut inputs = scriberig_agent_runtime::context::BranchExecutionContext::new(
        session.to_string(),
        branch.to_string(),
        node.title.clone(),
        node.purpose.clone(),
    )
    .with_project_scope(ProjectSemanticScope {
        project: fixture.project.to_string(),
        snapshot: fixture.snapshot.to_string(),
        root: "source-root".to_owned(),
        selection: None,
    });
    for (index, candidate) in candidate_ids.into_iter().enumerate() {
        inputs = inputs.with_dependency_result(CompactResultContext {
            handle: format!("result:{plan}:{index}"),
            revision: None,
            summary: "dependency completed with a candidate".to_owned(),
            semantic_handles: vec![format!("candidate:{candidate}")],
        });
    }
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(&inputs);
    let worker_context = PlanWorkerContext { inputs, compiled };
    let context = ApplicationContext::empty();
    let providers = std::collections::BTreeMap::new();
    let executor = crate::native_tui_agent::NativePlanWorkerExecutor::new(
        &service, &context, session, &providers, store, None,
    );
    let execution = scriberig_app::ResolvedPlanExecution {
        provider_id: "local".to_owned(),
        model_id: "unused-for-integration".to_owned(),
        model: rpi_ai::Model::new(
            "unused-for-integration",
            "unused-for-integration",
            rpi_ai::Api::AnthropicMessages,
            "local",
            "https://example.test",
        ),
        uses_runtime_fallback: true,
        thinking_level: None,
    };
    executor.execute_with_context(&node, &claim, &execution, &worker_context)
}
