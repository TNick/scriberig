//! Implements the bounded provider/tool and nested-child execution loop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use scriberig_agent_core::Tool;
use scriberig_app::{ApplicationContext, OperationRegistry, OperationStatus};

use crate::orchestration::{
    BranchPhase, BranchRuntimeState, RuntimeEvent, RuntimeEventLog, RuntimeLimits,
};
use crate::{
    AgentError, AgentMessage, AgentResult, ChildExecutionScope, ChildExecutor, ModelDecision,
    ModelProvider, OutputCallback,
};

#[path = "runtime/evidence.rs"]
mod evidence;
#[path = "runtime_helpers.rs"]
mod runtime_helpers;
use evidence::{child_evidence_record, inspect_evidence_record};
#[path = "runtime/cancellation.rs"]
mod cancellation;
pub use cancellation::cancellation_flag;
#[path = "runtime/configuration.rs"]
mod configuration;

/// Releases one shared execution-tree branch slot when a branch exits.
struct ActiveBranchGuard {
    /// Shared count for the root execution tree.
    count: Arc<std::sync::atomic::AtomicUsize>,
}

impl Drop for ActiveBranchGuard {
    /// Decrements the active branch count.
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Runs one model/tool loop through an explicit application context.
pub struct AgentRuntime<P, T> {
    provider: P,
    tool: T,
    registry: Option<OperationRegistry>,
    observed_operation: Option<scriberig_app::OperationId>,
    active_operation: Option<scriberig_app::OperationId>,
    next_operation: u64,
    event_log: Option<RuntimeEventLog>,
    output_callback: Option<OutputCallback>,
    branch_id: String,
    limits: RuntimeLimits,
}

impl<P, T> AgentRuntime<P, T>
where
    P: ModelProvider,
    T: Tool,
{
    /// Creates a runtime with a bounded tool-loop step count.
    ///
    /// # Parameters
    /// - `provider`: Model decision provider.
    /// - `tool`: Application-backed agent tool.
    /// - `step_limit`: Maximum number of decisions before failure.
    ///
    /// # Returns
    /// A configured agent runtime.
    #[must_use]
    pub fn new(provider: P, tool: T, step_limit: usize) -> Self {
        Self {
            provider,
            tool,
            registry: None,
            observed_operation: None,
            active_operation: None,
            next_operation: 1,
            event_log: None,
            output_callback: None,
            branch_id: "root".to_owned(),
            limits: RuntimeLimits {
                max_model_steps: step_limit,
                ..RuntimeLimits::default()
            },
        }
    }

    /// Executes one prompt until a final model response or failure.
    ///
    /// # Parameters
    /// - `self`: Runtime to execute.
    /// - `context`: Trusted application request context.
    /// - `prompt`: User instruction.
    /// - `cancelled`: Shared cancellation flag.
    ///
    /// # Returns
    /// Final model output and tool observations.
    ///
    /// # Errors
    /// Returns provider, tool, step-limit, or cancellation failures.
    pub fn run(
        &mut self,
        context: &ApplicationContext,
        prompt: impl Into<String>,
        cancelled: &AtomicBool,
    ) -> Result<AgentResult, AgentError> {
        self.run_inner(context, prompt.into(), cancelled, None, None)
    }

    /// Executes one prompt with a trusted nested-branch executor.
    ///
    /// # Parameters
    /// - `self`: Runtime to execute.
    /// - `context`: Trusted application request context.
    /// - `prompt`: User instruction.
    /// - `cancelled`: Shared cancellation flag.
    /// - `children`: Executor responsible for recursive child invocation.
    ///
    /// # Returns
    /// Final model output and compact child/tool observations.
    ///
    /// # Errors
    /// Returns provider, child, tool, step-limit, or cancellation failures.
    pub fn run_with_child_executor<C>(
        &mut self,
        context: &ApplicationContext,
        prompt: impl Into<String>,
        cancelled: &AtomicBool,
        children: &mut C,
    ) -> Result<AgentResult, AgentError>
    where
        C: ChildExecutor,
    {
        self.run_inner(context, prompt.into(), cancelled, None, Some(children))
    }

    /// Executes one prompt with a durable parent branch scope.
    ///
    /// # Parameters
    /// - `self`: Runtime to execute.
    /// - `scope`: Session and parent branch owning delegated children.
    /// - `context`: Trusted application request context.
    /// - `prompt`: User instruction.
    /// - `cancelled`: Shared cancellation flag.
    /// - `children`: Executor responsible for recursive child invocation.
    ///
    /// # Returns
    /// Final model output and compact child/tool observations.
    ///
    /// # Errors
    /// Returns provider, child, tool, step-limit, or cancellation failures.
    pub fn run_with_child_executor_at<C>(
        &mut self,
        scope: &ChildExecutionScope,
        context: &ApplicationContext,
        prompt: impl Into<String>,
        cancelled: &AtomicBool,
        children: &mut C,
    ) -> Result<AgentResult, AgentError>
    where
        C: ChildExecutor,
    {
        self.run_inner(
            context,
            prompt.into(),
            cancelled,
            Some(scope),
            Some(children),
        )
    }

    /// Runs the bounded loop with an optional nested-branch boundary.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `prompt`: Initial user prompt.
    /// - `cancelled`: Cancellation flag.
    /// - `scope`: Optional durable parent branch identity.
    /// - `children`: Optional child executor.
    ///
    /// # Returns
    /// Completed agent output and bounded observations.
    ///
    /// # Errors
    /// Returns the first provider, tool, child, cancellation, or budget error.
    #[allow(clippy::too_many_lines)]
    fn run_inner(
        &mut self,
        context: &ApplicationContext,
        prompt: String,
        cancelled: &AtomicBool,
        scope: Option<&ChildExecutionScope>,
        mut children: Option<&mut dyn ChildExecutor>,
    ) -> Result<AgentResult, AgentError> {
        let mut state = BranchRuntimeState::new(self.branch_id.clone());
        let branch_started = std::time::Instant::now();
        state.depth = scope.as_ref().map_or(0, |value| value.depth);
        if state.depth > self.limits.max_depth {
            self.emit(BranchPhase::BudgetExceeded);
            self.emit_event(&RuntimeEvent::branch_execution_failed(
                &self.branch_id,
                "branch depth budget",
            ));
            return Err(AgentError::BudgetExceeded {
                kind: "branch depth",
                limit: self.limits.max_depth,
            });
        }
        let _active_guard = self.acquire_active_branch(scope)?;
        self.emit(BranchPhase::Created);
        self.emit_event(&RuntimeEvent::branch_execution_started(&self.branch_id));
        self.emit(BranchPhase::CompilingContext);
        self.emit(BranchPhase::AwaitingModel);
        let operation = self.start_operation()?;
        self.emit_event(&RuntimeEvent::phase_changed(
            &self.branch_id,
            BranchPhase::AwaitingModel,
        ));
        let mut messages = vec![AgentMessage::User(prompt)];
        let mut tool_evidence = Vec::new();
        let mut model_requests = Vec::new();
        let mut activity_ordinal = 0;
        loop {
            if cancelled.load(Ordering::Acquire) {
                self.emit(BranchPhase::Cancelled);
                self.emit_event(&RuntimeEvent::branch_execution_cancelled(&self.branch_id));
                self.finish_operation(operation.as_ref(), OperationStatus::Cancelled);
                return Err(AgentError::Cancelled);
            }
            self.reserve_model_step(operation.as_ref(), &mut state)?;
            if self
                .limits
                .max_total_duration
                .is_some_and(|limit| branch_started.elapsed() > limit)
            {
                self.fail_budget(
                    operation.as_ref(),
                    "total duration",
                    self.limits.max_total_duration.map_or(0, |limit| {
                        usize::try_from(limit.as_secs()).unwrap_or(usize::MAX)
                    }),
                )?;
            }
            self.emit_event(&RuntimeEvent::model_invocation_started(
                &self.branch_id,
                state.model_steps,
            ));
            let provider_started = std::time::Instant::now();
            let event_log = self.event_log.clone();
            let branch_id = self.branch_id.clone();
            let registry = self.registry.clone();
            let operation_id = operation.clone();
            let output_callback = self.output_callback.clone();
            let decision = self
                .provider
                .decide_with_output(&messages, &mut |text| {
                    if let Some(event_log) = &event_log {
                        event_log.push(RuntimeEvent::model_text_delta(&branch_id, text));
                    }
                    if let (Some(registry), Some(operation_id)) = (&registry, &operation_id) {
                        let _ = registry.publish_agent_output_delta(operation_id, text);
                    }
                    if let Some(output_callback) = &output_callback {
                        output_callback(text);
                    }
                })
                .map_err(|error| {
                    self.emit(BranchPhase::Failed);
                    self.emit_event(&RuntimeEvent::branch_execution_failed(
                        &self.branch_id,
                        "provider",
                    ));
                    self.finish_operation(operation.as_ref(), OperationStatus::Failed);
                    AgentError::Provider(error)
                })?;
            for mut request in self.provider.take_request_projections() {
                request.ordinal = activity_ordinal;
                activity_ordinal += 1;
                model_requests.push(request);
            }
            if provider_started.elapsed() > self.limits.max_provider_duration {
                self.fail_budget(
                    operation.as_ref(),
                    "provider duration",
                    usize::try_from(self.limits.max_provider_duration.as_secs())
                        .unwrap_or(usize::MAX),
                )?;
            }
            if cancelled.load(Ordering::Acquire) {
                self.emit(BranchPhase::Cancelled);
                self.emit_event(&RuntimeEvent::branch_execution_cancelled(&self.branch_id));
                self.finish_operation(operation.as_ref(), OperationStatus::Cancelled);
                return Err(AgentError::Cancelled);
            }
            match decision {
                ModelDecision::Final(output) => {
                    self.emit(BranchPhase::Completing);
                    self.emit_event(&RuntimeEvent::branch_result_produced(
                        &self.branch_id,
                        &output,
                    ));
                    self.finish_operation(operation.as_ref(), OperationStatus::Completed);
                    self.emit(BranchPhase::Completed);
                    self.emit_event(&RuntimeEvent::branch_execution_completed(&self.branch_id));
                    return Ok(AgentResult {
                        output,
                        messages,
                        tool_evidence,
                        model_requests,
                    });
                }
                ModelDecision::Tool(input) => {
                    let result = self.run_atomic_tool(
                        context,
                        input.clone(),
                        operation.as_ref(),
                        &mut state,
                    )?;
                    let mut evidence = inspect_evidence_record(&input, &result);
                    evidence.ordinal = activity_ordinal;
                    activity_ordinal += 1;
                    tool_evidence.push(evidence);
                    messages.push(AgentMessage::Tool(result));
                    self.emit(BranchPhase::AwaitingModel);
                }
                ModelDecision::NamedTool {
                    name,
                    arguments,
                    presentation_arguments,
                    presentation_label,
                } => {
                    let (result, evidence) = self.run_named_tool_with_evidence(
                        context,
                        &name,
                        &arguments,
                        &presentation_arguments,
                        presentation_label.as_deref(),
                        cancelled,
                        operation.as_ref(),
                        &mut state,
                    )?;
                    let mut evidence = evidence;
                    evidence.ordinal = activity_ordinal;
                    activity_ordinal += 1;
                    tool_evidence.push(evidence);
                    messages.push(AgentMessage::Tool(result));
                    self.emit(BranchPhase::AwaitingModel);
                }
                ModelDecision::Child(request) => {
                    let result = self.run_child_tool(
                        context,
                        scope.cloned(),
                        request.clone(),
                        cancelled,
                        operation.as_ref(),
                        &mut state,
                        &mut children,
                    )?;
                    let mut evidence = child_evidence_record(&request, &result);
                    evidence.ordinal = activity_ordinal;
                    activity_ordinal += 1;
                    tool_evidence.push(evidence);
                    messages.push(AgentMessage::Child(result));
                    self.emit(BranchPhase::AwaitingModel);
                }
            }
        }
    }
}
