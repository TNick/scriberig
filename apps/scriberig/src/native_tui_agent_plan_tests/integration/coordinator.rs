//! Runs candidate production and integration through the durable coordinator.

use super::*;
use scriberig_agent_runtime::plan_context::PlanWorkerContext;
use scriberig_agent_runtime::scheduler::PlanWorkerExecutor;
use scriberig_session::{
    BranchResult, BranchStatus, ModelAssignment, PlanNode, PlanNodeStatus, WorkerClaim,
};

type CandidateBackend = ApplicationBackendComposition<
    StoreInspectionBackend<Arc<SqliteStore>>,
    StoreOperationProvider<Arc<SqliteStore>>,
>;
type CandidateNative<'a> = crate::native_tui_agent::NativePlanWorkerExecutor<'a, CandidateBackend>;

/// Coordinates concurrent candidate workers and an explicit integration node.
#[test]
fn coordinator_integrates_disjoint_candidates_and_records_conflicts() {
    run_candidate_plan(false);
    run_candidate_plan(true);
}

/// Runs one two-candidate plan with either disjoint edits or a conflict.
///
/// - `conflict`: Whether both candidates update the same source path.
///
/// Unit after the completed merge or durable structured failure is verified.
#[allow(clippy::too_many_lines)] // Covers concurrent production, merge, and conflict persistence.
fn run_candidate_plan(conflict: bool) {
    let fixture = candidate_fixture();
    let parent = fixture.root.parent().expect("fixture parent");
    let project_service = scriberig_project::ProjectService::new(Arc::clone(&fixture.store));
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(parent.join("session.sqlite"))
            .expect("session store"),
    );
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&fixture.store)),
        StoreOperationProvider::new(Arc::clone(&fixture.store)),
    );
    let library = scriberig_app::model_library_from_json(None, "integration-model")
        .expect("worker model library");
    let service = ApplicationService::new(backend)
        .with_session_store(session_store.clone())
        .with_model_library(library.clone());
    let session_id = EntityId::new();
    let root_branch = EntityId::new();
    let mut root = Branch::new(root_branch, "root").expect("root branch");
    root.status = BranchStatus::Completed;
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_plan").expect("plan capability"),
            CapabilityId::new("session_branch_tree").expect("session capability"),
        ]),
        ..ApplicationContext::empty()
    };
    SessionBackend::save_session(
        &service,
        &context,
        &Session {
            id: session_id,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection::default(),
        },
    )
    .expect("save session");
    let assignment = Some(ModelAssignment {
        profile: "integration-model-profile".to_owned(),
        model: "integration-model".to_owned(),
        thinking_level: None,
    });
    let integration = Some(ExpectedResult {
        kind: "project_candidate_merge".to_owned(),
        requires_handle: true,
    });
    let plan = SessionBackend::create_plan(
        &service,
        &context,
        &PlanCreateRequest {
            session: session_id,
            anchor: ContinuationAnchor::BranchHead {
                branch: root_branch,
            },
            goal: "propose and integrate two source candidates".to_owned(),
            nodes: vec![
                candidate_node("left", "Left candidate", assignment.clone()),
                candidate_node("right", "Right candidate", assignment.clone()),
                PlanNodeDraft {
                    key: "integration".to_owned(),
                    parent_index: None,
                    title: "Integrate candidates".to_owned(),
                    purpose: "Merge the exact candidate results from both dependencies.".to_owned(),
                    expected_result: integration,
                    semantic_scope: vec![
                        format!("project:{}", fixture.project),
                        format!("snapshot:{}", fixture.snapshot),
                    ],
                    assignment,
                },
            ],
            dependencies: vec![(0, 2), (1, 2)],
        },
    )
    .expect("create candidate plan");
    let plan = SessionBackend::plan_lifecycle(
        &service,
        &context,
        &PlanLifecycleRequest {
            session: session_id,
            plan: plan.id,
            expected_revision: plan.revision,
        },
        PlanLifecycleAction::Start,
    )
    .expect("start candidate plan");
    let providers = std::collections::BTreeMap::new();
    let native = crate::native_tui_agent::NativePlanWorkerExecutor::new(
        &service,
        &context,
        session_id,
        &providers,
        Arc::clone(&fixture.store),
        plan.coordinator_claim
            .as_ref()
            .and_then(|claim| claim.operation.clone()),
    );
    let executor = CandidatePlanExecutor {
        project: fixture.project,
        snapshot: fixture.snapshot,
        project_service: &project_service,
        native: &native,
        conflict,
        siblings: std::sync::Barrier::new(2),
    };
    scriberig_agent_runtime::coordinator::PlanCoordinator::run_until_idle(
        &service,
        &context,
        session_id,
        plan.id,
        plan.coordinator_claim
            .as_ref()
            .expect("coordinator claim")
            .id,
        &executor,
        &library,
        "integration-model",
        2,
        EntityId::new,
    )
    .expect("run candidate coordinator");
    let durable = session_store
        .load(session_id)
        .expect("load durable session")
        .expect("session exists");
    let completed = durable
        .plans
        .iter()
        .find(|candidate| candidate.id == plan.id)
        .expect("durable plan");
    let integration_node = completed
        .nodes
        .iter()
        .find(|node| node.title == "Integrate candidates")
        .expect("integration node");
    if conflict {
        assert_eq!(integration_node.status, PlanNodeStatus::Failed);
        let failure = durable
            .branches
            .iter()
            .find(|branch| branch.id == integration_node.branch.expect("failed branch"))
            .and_then(|branch| branch.result.as_ref())
            .expect("durable integration failure");
        assert!(
            failure
                .result_handles
                .contains(&"failure-code:candidate_merge_conflict".to_owned())
        );
        assert!(
            failure
                .summary
                .as_deref()
                .is_some_and(|summary| summary.contains("notes.txt"))
        );
    } else {
        assert_eq!(completed.status, scriberig_session::PlanStatus::Completed);
        assert_eq!(integration_node.status, PlanNodeStatus::Completed);
        let result = durable
            .branches
            .iter()
            .find(|branch| branch.id == integration_node.branch.expect("integration branch"))
            .and_then(|branch| branch.result.as_ref())
            .expect("integration result");
        let merged = result
            .result_handles
            .iter()
            .find_map(|handle| handle.strip_prefix("candidate:"))
            .expect("merged candidate");
        let candidate = project_service
            .candidate(merged)
            .expect("read merged candidate")
            .expect("candidate exists");
        assert_eq!(candidate.effects.len(), 2);
    }
    assert_eq!(
        project_service
            .workspace(fixture.project)
            .expect("project workspace")
            .snapshot,
        fixture.snapshot
    );
    drop(native);
    drop(service);
    drop(session_store);
    drop(project_service);
    drop(fixture.store);
    std::fs::remove_dir_all(parent).expect("remove plan fixture");
}

/// Creates one source-candidate worker node with a validated model assignment.
///
/// - `key`: Stable plan-local reference.
/// - `title`: Durable worker display label and deterministic fixture target.
/// - `assignment`: Model-library assignment used by scheduler resolution.
///
/// A structured source candidate proposal node.
/// # Returns
/// The candidate proposal draft.
fn candidate_node(key: &str, title: &str, assignment: Option<ModelAssignment>) -> PlanNodeDraft {
    PlanNodeDraft {
        key: key.to_owned(),
        parent_index: None,
        title: title.to_owned(),
        purpose: format!("Propose a candidate for the {key} dependency."),
        expected_result: Some(ExpectedResult {
            kind: "project_candidate".to_owned(),
            requires_handle: true,
        }),
        semantic_scope: Vec::new(),
        assignment,
    }
}

/// Runs deterministic candidate workers and delegates merge nodes to production.
struct CandidatePlanExecutor<'a> {
    /// Project receiving isolated candidate proposals.
    project: scriberig_core::ProjectId,
    /// Exact immutable source snapshot shared by candidate branches.
    snapshot: SnapshotId,
    /// Shared owner of source candidates and merge behavior.
    project_service: &'a scriberig_project::ProjectService,
    /// Production executor for the special semantic merge node.
    native: &'a CandidateNative<'a>,
    /// Whether both workers should conflict on one target.
    conflict: bool,
    /// Confirms both sibling candidate workers started before either proposal.
    siblings: std::sync::Barrier,
}

impl PlanWorkerExecutor for CandidatePlanExecutor<'_> {
    /// Rejects calls that omit the plan's exact dependency context.
    ///
    /// - `_node`: Durable candidate worker.
    /// - `_claim`: Worker branch claim.
    /// - `_execution`: Resolved provider assignment.
    ///
    /// Never returns a worker result.
    /// # Returns
    /// The result is always an error.
    /// # Errors
    /// Returns an error when exact context is unavailable.
    fn execute(
        &self,
        _node: &PlanNode,
        _claim: &WorkerClaim,
        _execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        Err("candidate workers require exact context".to_owned())
    }

    /// Produces isolated candidates or runs the native dependency integration.
    ///
    /// - `node`: Durable worker identity and semantic result contract.
    /// - `claim`: Exclusive branch and claim identity.
    /// - `execution`: Resolved model assignment from the existing library.
    /// - `context`: Exact branch, anchor, and dependency result context.
    ///
    /// A candidate result or production merge result on the claimed branch.
    /// # Returns
    /// The worker result.
    /// # Errors
    /// Returns a structured candidate or integration failure.
    fn execute_with_context(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
        context: &PlanWorkerContext,
    ) -> Result<BranchResult, String> {
        if node
            .expected_result
            .as_ref()
            .is_some_and(|expected| expected.kind == "project_candidate_merge")
        {
            return self
                .native
                .execute_with_context(node, claim, execution, context);
        }
        self.siblings.wait();
        let left = node.title == "Left candidate";
        let (path, candidate_content): (&str, &[u8]) = if left || self.conflict {
            (
                "notes.txt",
                if left {
                    b"left plan candidate\n"
                } else {
                    b"right plan candidate\n"
                },
            )
        } else {
            ("Cargo.lock", b"version = 4\n# plan candidate\n")
        };
        let candidate = self
            .project_service
            .propose_project_change(
                self.project,
                self.snapshot,
                vec![ProjectSourceChange {
                    path: path.to_owned(),
                    kind: ProjectSourceChangeKind::Update,
                    content: Some(candidate_content.to_vec()),
                }],
            )
            .map_err(|error| format!("{}: {}", error.code, error.message))?;
        Ok(BranchResult {
            id: EntityId::new(),
            branch: claim.branch,
            status: BranchStatus::Completed,
            summary: Some(format!("proposed candidate {}", candidate.id)),
            result_kind: Some("project_candidate".to_owned()),
            candidate_context: None,
            references: Vec::new(),
            result_handles: vec![
                format!("candidate:{}", candidate.id),
                format!("project:{}", candidate.project),
                format!("snapshot:{}", candidate.base_snapshot),
            ],
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        })
    }
}
