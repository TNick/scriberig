//! Verifies application-backed plan coordination and durable result ordering.

use scriberig_agent_runtime::{coordinator::PlanCoordinator, scheduler::PlanWorkerExecutor};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, CapabilityContext, CapabilityId, OperationProvider,
    PlanLifecycleRequest, SessionBackend, SessionRequest, StoreOperationProvider,
};
use scriberig_core::{EntityId, ProjectId, RevisionId};
use scriberig_session::{
    Branch, BranchResult, BranchStatus, CacheDisposition, ContinuationAnchor, ExecutionActivity,
    HumanInput, ModelInvocation, ModelOutput, Plan, PlanNode, PlanNodeParent, PlanNodeStatus,
    PlanStatus, Session, ToolActivityKind, ToolInvocation, ToolResult, Turn, WorkerClaim,
};
use scriberig_store::{RootStore, SnapshotChange, SnapshotStore};
use scriberig_store_memory::MemoryStore;

/// Supplies merge operations for deterministic candidate integration tests.
struct Backend {
    /// Immutable snapshot store shared with candidate-producing workers.
    store: std::sync::Arc<MemoryStore>,
}

impl ApplicationBackend for Backend {
    /// Returns unsupported for semantic operations unused by this test.
    /// # Parameters
    /// - `context`: Input for this operation.
    /// - `request`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        if let ApplicationRequest::Merge(merge) = request {
            return StoreOperationProvider::new(std::sync::Arc::clone(&self.store))
                .merge(context, merge)
                .map(ApplicationResult::Merge);
        }
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture operation".to_owned(),
        })
    }
}

/// Synchronizes assigned siblings and returns compact durable results.
struct FakeWorkers<'a> {
    /// Barrier reached when both initially ready sibling workers have entered.
    barrier: std::sync::Barrier,
    /// Number of worker invocations observed across coordinator passes.
    started: std::sync::atomic::AtomicUsize,
    /// Records that the dependent worker received compact dependency context.
    dependency_context_seen: std::sync::atomic::AtomicBool,
    /// Reads durable state while a provider call is in progress.
    read_durable: &'a (dyn Fn() -> Session + Sync),
    /// Persists one simulated worker's private branch history.
    save_durable: &'a (dyn Fn(Session) + Sync),
}

impl PlanWorkerExecutor for FakeWorkers<'_> {
    /// Returns one exact completed result for the claimed worker branch.
    /// # Parameters
    /// - `node`: Input for this operation.
    /// - `claim`: Input for this operation.
    /// - `execution`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    #[allow(clippy::too_many_lines)] // One deterministic worker run checks durable isolation.
    fn execute(
        &self,
        node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        let durable = (self.read_durable)();
        let branch = durable
            .branches
            .iter()
            .find(|branch| branch.id == claim.branch)
            .expect("worker branch persisted before provider execution");
        assert_eq!(branch.status, BranchStatus::Active);
        assert_eq!(
            branch.continuation,
            Some(scriberig_session::ContinuationProvenance {
                anchor: ContinuationAnchor::BranchHead {
                    branch: EntityId::from_bytes([2; 16]),
                },
                origin: scriberig_session::ContinuationOrigin::PlanExecution,
            })
        );
        if node.title == "analyze" {
            let mut durable = (self.read_durable)();
            let branch = durable
                .branches
                .iter_mut()
                .find(|branch| branch.id == claim.branch)
                .expect("analyze branch");
            let private_input = EntityId::from_bytes([81; 16]);
            let private_model = EntityId::from_bytes([82; 16]);
            let private_output = EntityId::from_bytes([83; 16]);
            let private_tool = EntityId::from_bytes([84; 16]);
            let private_result = EntityId::from_bytes([85; 16]);
            branch.push_turn(Turn {
                id: private_input,
                ordinal: 0,
                human: Some(HumanInput {
                    text: "PRIVATE_DEPENDENCY_TRANSCRIPT inspect declarations".to_owned(),
                }),
                agent: None,
                tool_invocation: None,
                tool_result: None,
            });
            branch.model_invocations.push(ModelInvocation {
                id: private_model,
                branch: claim.branch,
                ordinal: 1,
                agent_profile: None,
                context_fingerprint: None,
                provider_profile: Some("private-worker-profile".to_owned()),
                model: Some("private-worker-model".to_owned()),
                model_profile: Some("private-worker-profile".to_owned()),
                provider_model_id: None,
                thinking_level: None,
                request_fingerprint: None,
                cache: CacheDisposition::NotUsed,
                output: Some(private_output),
                request: None,
            });
            branch.model_outputs.push(ModelOutput {
                id: private_output,
                invocation: private_model,
                text: Some("private intermediate summary".to_owned()),
                structured_output: None,
                finish_reason: Some("tool_call".to_owned()),
                usage: None,
            });
            branch.tool_invocations.push(ToolInvocation {
                id: private_tool,
                name: "semantic.inspect_fixture".to_owned(),
                input: b"PRIVATE_TOOL_TRANSCRIPT inspect entity".to_vec(),
                parent_model_invocation: Some(private_model),
                presentation_input: None,
                presentation_label: None,
                category: Some(ToolActivityKind::SemanticMutation),
            });
            branch.tool_results.push(ToolResult {
                id: private_result,
                invocation: private_tool,
                output: b"PRIVATE_TOOL_RESULT intermediate entity revision".to_vec(),
                failed: false,
                presentation_preview: None,
            });
            branch.activity.extend([
                ExecutionActivity::HumanInput(private_input),
                ExecutionActivity::ModelInvocation(private_model),
                ExecutionActivity::ModelOutput(private_output),
                ExecutionActivity::ToolInvocation(private_tool),
                ExecutionActivity::ToolResult(private_result),
            ]);
            (self.save_durable)(durable);
        }
        if self
            .started
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            < 2
        {
            self.barrier.wait();
        }
        let mut result_handles = vec![
            format!(
                "profile:{}",
                execution.model_profile.as_deref().unwrap_or("default")
            ),
            format!("model:{}", execution.model.as_deref().unwrap_or("fallback")),
        ];
        if node.title == "analyze" {
            result_handles.push("entity:api-contract".to_owned());
        }
        Ok(BranchResult {
            id: EntityId::from_bytes([claim.id.as_bytes()[0].wrapping_add(80); 16]),
            branch: claim.branch,
            status: BranchStatus::Completed,
            summary: Some(format!("{} completed", node.title)),
            result_kind: Some("worker-summary".to_owned()),
            candidate_context: None,
            references: Vec::new(),
            result_handles,
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        })
    }

    /// Verifies integration work receives scope and result handles without transcripts.
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
        if node.title == "integrate" {
            assert!(context.compiled.canonical.contains("dependency_result"));
            assert!(context.compiled.canonical.contains("profile:alpha"));
            assert!(context.compiled.canonical.contains("entity:api-contract"));
            assert!(context.compiled.canonical.contains("Exact semantic scope"));
            assert!(
                !context
                    .compiled
                    .canonical
                    .contains("PRIVATE_DEPENDENCY_TRANSCRIPT")
            );
            assert!(
                !context
                    .compiled
                    .canonical
                    .contains("PRIVATE_TOOL_TRANSCRIPT")
            );
            assert!(!context.compiled.canonical.contains("PRIVATE_TOOL_RESULT"));
            assert!(
                !context
                    .compiled
                    .canonical
                    .contains("private-worker-profile")
            );
            assert!(
                !context
                    .compiled
                    .canonical
                    .contains("private-worker-transcript")
            );
            self.dependency_context_seen
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        self.execute(node, claim, execution)
    }
}

#[path = "coordinator_parts/additional.rs"]
mod coordinator_additional;
