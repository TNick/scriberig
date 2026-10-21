//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies plan integration merges candidates without promoting project state.
#[test]
#[allow(clippy::too_many_lines)] // The scenario asserts the full explicit integration boundary.
fn plan_integration_merges_candidates_without_promoting_project_state() {
    let store = std::sync::Arc::new(MemoryStore::new());
    let base = store.empty_snapshot();
    let service = ApplicationService::new(Backend {
        store: std::sync::Arc::clone(&store),
    });
    let session_id = EntityId::from_bytes([101; 16]);
    let root = EntityId::from_bytes([102; 16]);
    let plan_id = EntityId::from_bytes([103; 16]);
    let left_node = EntityId::from_bytes([104; 16]);
    let right_node = EntityId::from_bytes([105; 16]);
    let integrate_node = EntityId::from_bytes([106; 16]);
    let project = ProjectId::from_bytes([107; 16]);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("read grant"),
            CapabilityId::new("agent_plan").expect("plan grant"),
        ]),
        ..ApplicationContext::empty()
    };
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: root },
        "merge independent semantic changes",
    )
    .expect("plan");
    for (id, title) in [
        (left_node, "left"),
        (right_node, "right"),
        (integrate_node, "integrate"),
    ] {
        plan.add_node(PlanNode {
            id,
            parent: Some(PlanNodeParent::Branch(root)),
            title: title.to_owned(),
            purpose: title.to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: PlanNodeStatus::Planned,
            branch: None,
            result: None,
            superseded_by: None,
            assignment: None,
            claim: None,
            claim_generation: 0,
        })
        .expect("plan node");
    }
    plan.add_dependency(left_node, integrate_node)
        .expect("left dependency");
    plan.add_dependency(right_node, integrate_node)
        .expect("right dependency");
    plan.transition(PlanStatus::Planned).expect("planned");
    let revision = plan.revision;
    let mut session = Session {
        id: session_id,
        branches: vec![Branch::new(root, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    session.add_plan(plan).expect("attach plan");
    SessionBackend::save_session(&service, &context, &session).expect("save session");
    let started = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session: session_id,
                plan: plan_id,
                expected_revision: revision,
            },
        )
        .expect("start coordinator");
    let coordinator_claim = started
        .coordinator_claim
        .as_ref()
        .expect("coordinator claim")
        .id;
    let read_durable = || {
        SessionBackend::read_session(
            &service,
            &context,
            &SessionRequest {
                session: session_id,
            },
        )
        .expect("read session")
    };
    let workers = CandidateWorkers {
        store: std::sync::Arc::clone(&store),
        service: &service,
        context: &context,
        base,
        project,
        barrier: std::sync::Barrier::new(2),
        read_durable: &read_durable,
    };
    let mut next = 108_u8;
    let passes = PlanCoordinator::run_until_idle(
        &service,
        &context,
        session_id,
        plan_id,
        coordinator_claim,
        &workers,
        &runtime_fallback_models(),
        "fallback-model",
        2,
        || {
            next = next.saturating_add(1);
            EntityId::from_bytes([next; 16])
        },
    )
    .expect("run integration plan");

    assert_eq!(passes.len(), 2);
    assert_eq!(passes[0].workers.completed, vec![left_node, right_node]);
    assert_eq!(passes[1].workers.completed, vec![integrate_node]);
    assert!(
        passes[1].workers.results[0]
            .result_handles
            .iter()
            .any(|handle| handle.starts_with("merged_candidate:"))
    );
    let durable = read_durable();
    for title in ["left", "right"] {
        let branch = durable
            .branches
            .iter()
            .find(|branch| branch.title == title)
            .expect("worker branch persisted");
        let invocation = branch
            .model_invocations
            .first()
            .expect("worker model invocation persisted");
        assert_eq!(invocation.provider_id.as_deref(), Some("fallback-provider"));
        assert_eq!(
            invocation.provider_model_id.as_deref(),
            Some("fallback-model")
        );
        assert!(
            branch
                .activity
                .iter()
                .any(|activity| { *activity == ExecutionActivity::ModelInvocation(invocation.id) })
        );
        let request = invocation
            .request
            .as_ref()
            .expect("request detail persisted");
        assert_eq!(request.provider_id.as_deref(), Some("fallback-provider"));
        assert_eq!(request.model, "fallback-model");
    }
    assert!(
        store
            .read_root(&scriberig_core::RootName::new("main").expect("root"))
            .is_none()
    );
}
/// Executes candidate-producing workers and an explicit integration merge.
struct CandidateWorkers<'a> {
    /// Immutable store receiving worker candidate snapshots.
    store: std::sync::Arc<MemoryStore>,
    /// Shared application service used for the integration operation.
    service: &'a ApplicationService<Backend>,
    /// Trusted context used for the application merge request.
    context: &'a ApplicationContext,
    /// Common exact base shared by both sibling candidates.
    base: scriberig_core::SnapshotId,
    /// Project identity attached to candidate results.
    project: ProjectId,
    /// Barrier proving both candidate workers overlap.
    barrier: std::sync::Barrier,
    /// Reads the durable results produced by preceding scheduler passes.
    read_durable: &'a (dyn Fn() -> Session + Sync),
}

impl PlanWorkerExecutor for CandidateWorkers<'_> {
    /// Produces one candidate result or explicitly merges fulfilled dependencies.
    /// # Parameters
    /// - `node`: Input for this operation.
    /// - `claim`: Input for this operation.
    /// - `_execution`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        _execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        self.barrier.wait();
        let index = if node.title == "left" { 1 } else { 2 };
        let entity = EntityId::from_bytes([index; 16]);
        let candidate = self
            .store
            .apply_changes(
                self.base,
                &[SnapshotChange::Set(entity, RevisionId::new([index; 32]))],
            )
            .map_err(|error| format!("{error:?}"))?;
        Ok(BranchResult {
            id: EntityId::from_bytes([claim.id.as_bytes()[0].wrapping_add(60); 16]),
            branch: claim.branch,
            status: BranchStatus::Completed,
            summary: Some(format!("{} candidate", node.title)),
            result_kind: Some("candidate".to_owned()),
            candidate_context: Some(scriberig_session::BranchCandidateContext {
                project: self.project,
                base_snapshot: self.base,
                candidate_snapshot: candidate,
            }),
            references: Vec::new(),
            result_handles: vec![format!("candidate_snapshot:{candidate}")],
            touched: vec![entity],
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        })
    }

    /// Merges the two exact dependency candidates through the app boundary.
    /// # Parameters
    /// - `node`: Input for this operation.
    /// - `claim`: Input for this operation.
    /// - `execution`: Input for this operation.
    /// - `context`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute_with_context(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
        context: &scriberig_agent_runtime::plan_context::PlanWorkerContext,
    ) -> Result<BranchResult, String> {
        if node.title != "integrate" {
            return self.execute(node, claim, execution);
        }
        let durable = (self.read_durable)();
        let candidates = durable
            .branches
            .iter()
            .filter_map(|branch| branch.result.as_ref()?.candidate_context)
            .collect::<Vec<_>>();
        if candidates.len() != 2 || context.inputs.dependency_results.len() != 2 {
            return Err("integration requires two explicit dependency results".to_owned());
        }
        let merge = self
            .service
            .execute(
                self.context,
                &ApplicationRequest::Merge(scriberig_app::MergeRequest {
                    base: candidates[0].base_snapshot,
                    left: candidates[0].candidate_snapshot,
                    right: candidates[1].candidate_snapshot,
                }),
            )
            .map_err(|error| format!("{error:?}"))?;
        let ApplicationResult::Merge(merge) = merge else {
            return Err("app boundary returned an incompatible merge result".to_owned());
        };
        if !merge.conflicts.is_empty() {
            return Err("disjoint worker candidates unexpectedly conflicted".to_owned());
        }
        let merged = merge
            .result
            .ok_or_else(|| "explicit merge omitted its candidate snapshot".to_owned())?;
        let conflict_left = self
            .store
            .apply_changes(
                self.base,
                &[SnapshotChange::Set(
                    EntityId::from_bytes([9; 16]),
                    RevisionId::new([91; 32]),
                )],
            )
            .map_err(|error| format!("{error:?}"))?;
        let conflict_right = self
            .store
            .apply_changes(
                self.base,
                &[SnapshotChange::Set(
                    EntityId::from_bytes([9; 16]),
                    RevisionId::new([92; 32]),
                )],
            )
            .map_err(|error| format!("{error:?}"))?;
        let conflict = self
            .service
            .execute(
                self.context,
                &ApplicationRequest::Merge(scriberig_app::MergeRequest {
                    base: self.base,
                    left: conflict_left,
                    right: conflict_right,
                }),
            )
            .map_err(|error| format!("{error:?}"))?;
        let ApplicationResult::Merge(conflict) = conflict else {
            return Err("app boundary returned an incompatible conflict result".to_owned());
        };
        if conflict.conflicts.is_empty() || conflict.result.is_some() {
            return Err("same-target changes did not produce a structured conflict".to_owned());
        }
        Ok(BranchResult {
            id: EntityId::from_bytes([claim.id.as_bytes()[0].wrapping_add(60); 16]),
            branch: claim.branch,
            status: BranchStatus::Completed,
            summary: Some("explicit merge candidate created".to_owned()),
            result_kind: Some("merged_candidate".to_owned()),
            candidate_context: Some(scriberig_session::BranchCandidateContext {
                project: self.project,
                base_snapshot: self.base,
                candidate_snapshot: merged,
            }),
            references: Vec::new(),
            result_handles: vec![format!("merged_candidate:{merged}")],
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        })
    }

    /// Returns provider requests separately for coordinator-owned persistence.
    /// # Parameters
    /// - `node`: Claimed plan node.
    /// - `claim`: Exclusive branch claim.
    /// - `execution`: Upstream-resolved provider/model pair.
    /// - `context`: Exact compiled worker context.
    /// # Returns
    /// The semantic result with bounded request evidence.
    /// # Errors
    /// Returns worker failures from the underlying execution.
    fn execute_with_context_and_requests(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
        context: &scriberig_agent_runtime::plan_context::PlanWorkerContext,
    ) -> Result<PlanWorkerExecutionResult, String> {
        let result = self.execute_with_context(node, claim, execution, context)?;
        let model_requests = if node.title == "integrate" {
            Vec::new()
        } else {
            vec![scriberig_session::ModelRequestProjection {
                ordinal: 0,
                model: execution.model_id.clone(),
                provider_id: Some(execution.provider_id.clone()),
                provider_profile: None,
                thinking_level: execution
                    .thinking_level
                    .map(|level| level.as_str().to_owned()),
                profile: None,
                active_components: Vec::new(),
                context_fingerprint: None,
                request_fingerprint: None,
                model_execution_configuration: None,
                messages: Vec::new(),
                tools: Vec::new(),
                applied_settings: None,
                applied_settings_fingerprint: None,
                truncated: false,
            }]
        };
        Ok(PlanWorkerExecutionResult {
            result,
            model_requests,
            tool_evidence: Vec::new(),
        })
    }
}
